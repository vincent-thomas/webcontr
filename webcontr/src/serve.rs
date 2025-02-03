use std::{
  future::{Future, IntoFuture}, io, pin::Pin, sync::Arc, task::{Context, Poll}
};

use futures_util::{SinkExt, StreamExt};
use tokio::{
  net::TcpListener,
  sync::watch,
  time::{sleep, Duration, Instant, Sleep},
};
use tokio_util::task::TaskTracker;
use tower::Service;

use crate::{
  transport::{
    frame::{ResponseErrorKind, ResponseFrame},
    tcp,
  }, FrozenServer
};

pub struct ServerServe {
  pub(crate) server: FrozenServer,
  pub(crate) listener: TcpListener,
  pub(crate) timeout: Option<Duration>,

  #[cfg(feature = "tls")]
  pub(crate) tls_paths: crate::tls::TLSPaths,
}

impl ServerServe {
  pub fn with_timeout(mut self, dur: Duration) -> Self {
    self.timeout = Some(dur);
    self
  }
}

impl IntoFuture for ServerServe {
  type Output = io::Result<()>;

  type IntoFuture = Pin<Box<dyn Future<Output = Self::Output>>>;

  fn into_future(self) -> Self::IntoFuture {
    let (shutdown_tx, mut shutdown_rx) = watch::channel::<bool>(false);
    let task_tracker = TaskTracker::default();

    task_tracker.spawn(async move {
      tokio::signal::ctrl_c().await.unwrap();
      shutdown_tx.send(true).unwrap();

      eprintln!("Received Ctrl+C signal");
    });

    #[cfg(feature = "tls")]
    let acceptor = {
      let config = self.tls_paths.clone().serverconfig_from_paths();
      tokio_rustls::TlsAcceptor::from(Arc::new(config))
    };

    Box::pin(async move {
      loop {
        let (stream, _) = tokio::select! {
            listener = self.listener.accept() => listener?,
            _ = shutdown_rx.changed() => {
                task_tracker.close();
                task_tracker.wait().await;
                return Ok(())
            },
        };

        let mut server = self.server.clone();

        #[cfg(feature = "tls")]
        let acceptor = acceptor.clone();

        task_tracker.spawn(async move {
           let stream = {
             #[cfg(feature = "tls")] { acceptor.accept(stream).await.unwrap() }
             #[cfg(not(feature = "tls"))] { stream }
           };
           let mut transport = tcp::request_transport(stream);
          
            let value;
            loop {
              value = match transport.next().await {
                Some(value) => match value {
                  Ok(value) => value,
                  Err(_) => return,
                },
                None => continue,
              };
              break;
            }
            let mut transport = tcp::response_transport(transport.into_inner());

            match server.query(value.command.as_str()) {
              Some(service_ref) => {
                  let mut service = service_ref.clone();
                match ServeTaskFuture::new(self.timeout, service.call(value.arguments)).await 
                {
                    Ok(value) => match value {
                        Ok(bytes) => transport.send(ResponseFrame::Payload(bytes)).await.expect("webcontr internal error: failed to send ResponseFrame::Payload"),
                        Err(err) => transport.send(ResponseFrame::Error(err)).await.expect("webcontr internal error: failed to send ResponseFrame::Error")
                    },
                    Err(()) => transport.send(ResponseFrame::Error(ResponseErrorKind::Timeout)).await.expect("webcontr internal error: failed to send ResponseFrame::Error(ResponseErrorKind::Timeout)"),
                }
              }
              _ => {
                transport
                  .send(ResponseFrame::Error(ResponseErrorKind::MethodNotFound))
                  .await
                  .expect("webcontr internal error: failed to send ResponseFrame::Error(ResponseErrorKind::MethodNotFound)");
              }
            };
        });
      }
    })
  }
}

pin_project_lite::pin_project! {
    pub struct ServeTaskFuture<F> {
        #[pin]
      future: F,
      timeout: Option<Pin<Box<Sleep>>>,
      start: Instant,
      duration: Option<Duration>,
    }
}

//#[cfg(test)]
//static_assertions::assert_impl_all!(ServeTaskFuture<_>: Send);

impl<F> ServeTaskFuture<F> {
  pub fn new(duration: Option<Duration>, future: F) -> Self {
    ServeTaskFuture {
      future,
      timeout: duration.map(|d| Box::pin(sleep(d))),
      start: Instant::now(),
      duration,
    }
  }
}

impl<F: Send + Future> Future for ServeTaskFuture<F> {
  type Output = Result<F::Output, ()>;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.project();

    if let Some(ref mut timeout) = this.timeout {
      if timeout.as_mut().poll(cx).is_ready() {
        return Poll::Ready(Err(()));
      }
    }

    match this.future.poll(cx) {
      Poll::Ready(val) => Poll::Ready(Ok(val)),
      Poll::Pending => Poll::Pending,
    }
  }
}
