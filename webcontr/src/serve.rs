use std::{
  future::{Future, IntoFuture},
  io,
  pin::Pin,
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
  net::TcpListener,
  task,
  time::{timeout, Duration},
};

use crate::{
  transport::{
    frame::{ResponseErrorKind, ResponseFrame},
    tcp,
  },
  FrozenServer, ServeError,
};

pub struct ServerServe {
  pub(crate) server: FrozenServer,
  pub(crate) listener: TcpListener,
  pub(crate) timeout: Duration,
}

impl IntoFuture for ServerServe {
  type Output = io::Result<()>;

  type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

  fn into_future(self) -> Self::IntoFuture {
    Box::pin(async move {
      loop {
        let (stream, _) = self.listener.accept().await?;

        let (read, mut write) = stream.into_split();

        let server = self.server.clone();
        task::spawn(timeout(self.timeout, async move {
          let mut transport = tcp::request_transport(read);

          while let Some(value) = transport.next().await {
            let mut transport = tcp::response_transport(&mut write);

            let value = match value {
              Ok(value) => value,
              Err(_) => break,
            };

            let key = value.command.as_str();

            match server.query(key) {
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
              }
            };
          }
        }));
      }
    })
  }
}
