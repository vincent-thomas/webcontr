use crate::{serve::ServerServe, Serve, ServiceName};
use std::{collections::HashMap, sync::Arc};
use tokio::net::TcpListener;

#[derive(Default)]
pub struct Server {
  hash: HashMap<&'static str, Box<dyn Serve>>,
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
  pub(crate) fn query(&self, cmd: &str) -> Option<&dyn Serve> {
    self.inner.query(cmd)
  }
}

#[cfg(test)]
static_assertions::assert_impl_all!(FrozenServer: Send, Sync);

impl Server {
  pub fn add_service<S>(mut self, service: S) -> Self
  where
    S: Serve + ServiceName + 'static,
  {
    self.hash.insert(service.name(), Box::new(service));
    self
  }

  pub fn serve(self, tcp_listener: TcpListener) -> ServerServe {
    ServerServe {
      server: FrozenServer::from(self),
      listener: tcp_listener,
      timeout: None,
    }
  }

  pub(crate) fn query(&self, cmd: &str) -> Option<&dyn Serve> {
    self.hash.get(cmd).map(|v| &**v)
  }
}
