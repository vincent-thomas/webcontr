pub mod prelude;
pub mod serve;
mod server;
pub mod transport;
use std::io;

pub use server::*;

pub use async_trait::async_trait;

use bytes::Bytes;
use transport::frame::ResponseErrorKind;
pub use webcontr_macros::service;

#[async_trait]
pub trait Serve: Send + Sync {
  async fn serve(&self, req: Bytes) -> Result<Bytes, ServeError>;
}

pub trait ServiceName {
  fn name(&self) -> &'static str;
}

#[cfg(test)]
static_assertions::assert_obj_safe!(Serve, ServiceName);

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
  #[error("io error: {0}")]
  IoError(io::Error),
  #[error("server error: {0}")]
  ServerError(ResponseErrorKind),
  #[error("encoding error: {0}")]
  EncodingError(Box<bincode::ErrorKind>),
}

#[derive(Debug)]
pub enum ServeError {
  MethodNotFound,
  InvalidRequest,
}
