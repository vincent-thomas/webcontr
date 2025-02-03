#[cfg(feature = "tls")]
use crate::tls::TLSPaths;
use crate::{
  serve::ServerServe, transport::frame::ResponseErrorKind,
  utils::BoxCloneService, ServiceName,
};
use bytes::Bytes;
use std::{collections::HashMap, future::Future, pin::Pin, sync::Arc};
use tokio::net::TcpListener;
use tower::Service;

type ServiceFuture =
  Pin<Box<dyn Future<Output = Result<Bytes, ResponseErrorKind>> + Send>>;

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
