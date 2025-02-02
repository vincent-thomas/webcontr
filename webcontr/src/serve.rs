use std::{
  future::{Future, IntoFuture},
  io,
  pin::Pin,
};

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;

use crate::{
  transport::tcp::{
    self,
    frame::{ResponseErrorKind, ResponseFrame},
  },
  ServeError, Server,
};

pub struct ServerServe {
  pub(crate) server: Server,
  pub(crate) listener: TcpListener,
}

impl IntoFuture for ServerServe {
  type Output = io::Result<()>;

  type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

  fn into_future(self) -> Self::IntoFuture {
    Box::pin(async move {
      loop {
        let (stream, _) = self.listener.accept().await?;

        let mut transport = tcp::request_transport(stream);

        let Some(Ok(value)) = transport.next().await else {
          continue;
        };
        let mut transport = tcp::response_transport(transport.into_inner());

        let key = value.command.as_str();

        match self.server.hash.get(key) {
          Some(service) => {
            match service.serve(value.arguments).await {
              Ok(value) => {
                transport.send(ResponseFrame::Payload(value)).await.unwrap()
              }
              Err(err) => {
                let err = match err {
                  ServeError::MethodNotFound => {
                    ResponseErrorKind::MethodNotFound
                  }
                  ServeError::InvalidRequest => {
                    ResponseErrorKind::InvalidRequest
                  }
                };
                transport.send(ResponseFrame::Error(err)).await.unwrap()
              }
            };
          }
          _ => {
            transport
              .send(ResponseFrame::Error(ResponseErrorKind::MethodNotFound))
              .await
              .unwrap();
            continue;
          }
        };
      }
    })
  }
}
