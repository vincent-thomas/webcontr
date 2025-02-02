pub mod prelude;
pub mod serve;
pub mod transport;
use std::collections::HashMap;

pub use async_trait::async_trait;

use bytes::Bytes;
use serve::ServerServe;
use tokio::net::TcpListener;
pub use webcontr_macros::service;

#[async_trait]
pub trait Serve {
  async fn serve(&self, req: Bytes) -> Result<Bytes, ServeError>;
}

pub trait ServiceName {
  fn name(&self) -> &'static str;
}

#[derive(Debug)]
pub enum ServeError {
  MethodNotFound,
  InvalidRequest,
}

#[derive(Default)]
pub struct Server {
  hash: HashMap<&'static str, Box<dyn Serve>>,
}

impl Server {
  pub fn add_service<S>(mut self, service: S) -> Self
  where
    S: Serve + ServiceName + 'static,
  {
    self.hash.insert(service.name(), Box::new(service));
    self
  }

  pub fn serve(self, tcp_listener: TcpListener) -> ServerServe {
    ServerServe { server: self, listener: tcp_listener }
  }
}
