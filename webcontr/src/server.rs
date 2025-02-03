#[cfg(feature = "tls")]
use crate::tls::TLSPaths;
use crate::{
  serve::ServerServe, transport::frame::ResponseErrorKind, Serve, ServiceName,
};
use bytes::Bytes;
use futures_util::future::BoxFuture;
use std::{
  collections::HashMap,
  future::Future,
  pin::Pin,
  sync::Arc,
  task::{Context, Poll},
};
use tokio::net::TcpListener;
use tower::Service;

type ServiceFuture =
  Pin<Box<dyn Future<Output = Result<Bytes, ResponseErrorKind>> + Send>>;

pub trait CloneService<R>: Service<R> {
  fn clone_box(
    &self,
  ) -> Box<
    dyn CloneService<
        R,
        Response = Self::Response,
        Error = Self::Error,
        Future = Self::Future,
      > + Send
      + Sync,
  >;
}
impl<R, T> CloneService<R> for T
where
  T: Service<R> + Send + Sync + Clone + 'static,
{
  fn clone_box(
    &self,
  ) -> Box<
    dyn CloneService<
        R,
        Response = T::Response,
        Error = T::Error,
        Future = T::Future,
      > + Send
      + Sync,
  > {
    Box::new(self.clone())
  }
}
impl<T, U, E> Clone for BoxCloneService<T, U, E> {
  fn clone(&self) -> Self {
    Self(self.0.clone_box())
  }
}

pub struct BoxCloneService<T, U, E>(
  Box<
    dyn CloneService<
        T,
        Response = U,
        Error = E,
        Future = BoxFuture<'static, Result<U, E>>,
      > + Send
      + Sync,
  >,
);

impl<T, U, E> BoxCloneService<T, U, E> {
  pub(crate) fn new<S>(inner: S) -> Self
  where
    S: Service<T, Response = U, Error = E> + Clone + Send + Sync + 'static,
    S::Future: Send + 'static,
  {
    let inner = tower::ServiceExt::map_future(inner, |f| Box::pin(f) as _);
    BoxCloneService(Box::new(inner))
  }
}

impl<T, U, E> Service<T> for BoxCloneService<T, U, E> {
  type Response = U;
  type Error = E;
  type Future = BoxFuture<'static, Result<U, E>>;

  #[inline]
  fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), E>> {
    self.0.poll_ready(cx)
  }

  #[inline]
  fn call(&mut self, request: T) -> Self::Future {
    self.0.call(request)
  }
}

#[derive(Default)]
pub struct Server {
  pub hash:
    HashMap<&'static str, BoxCloneService<Bytes, Bytes, ResponseErrorKind>>,
}

#[derive(Clone)]
pub struct FrozenServer {
  inner: Arc<Server>,
}

impl From<Server> for FrozenServer {
  fn from(value: Server) -> Self {
    FrozenServer { inner: Arc::new(value) }
  }
}

impl FrozenServer {
  pub(crate) fn query(
    &mut self,
    cmd: &str,
  ) -> Option<&BoxCloneService<Bytes, Bytes, ResponseErrorKind>> {
    self.inner.hash.get(cmd)
  }
}

#[cfg(test)]
static_assertions::assert_impl_all!(FrozenServer: Send, Sync);

impl Server {
  pub fn add_service<S>(mut self, service: S) -> Self
  where
    S: Service<
        Bytes,
        Response = Bytes,
        Error = ResponseErrorKind,
        Future = ServiceFuture,
      >
      + ServiceName
      + 'static
      + Sync
      + Send
      + Clone,
  {
    self.hash.insert(service.name(), BoxCloneService::new(service));
    self
  }

  pub fn serve(
    self,
    tcp_listener: TcpListener,
    #[cfg(feature = "tls")] tls_paths: TLSPaths,
  ) -> ServerServe {
    ServerServe {
      server: FrozenServer::from(self),
      listener: tcp_listener,
      timeout: None,
      #[cfg(feature = "tls")]
      tls_paths,
    }
  }
}
