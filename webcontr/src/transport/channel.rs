use std::task::Poll;

use futures_util::{Sink, Stream};
use tokio::sync::mpsc::{self, error::SendError};

pub struct UnboundedChannel<Item, SinkItem> {
  sender: mpsc::UnboundedSender<SinkItem>,
  receiver: mpsc::UnboundedReceiver<Item>,
}

pub fn unbounded<SinkItem, Item>(
) -> (UnboundedChannel<SinkItem, Item>, UnboundedChannel<Item, SinkItem>) {
  let (sender1, receiver2) = mpsc::unbounded_channel();
  let (sender2, receiver1) = mpsc::unbounded_channel();
  (
    UnboundedChannel { sender: sender1, receiver: receiver1 },
    UnboundedChannel { sender: sender2, receiver: receiver2 },
  )
}

impl<Item, SinkItem> Stream for UnboundedChannel<Item, SinkItem> {
  type Item = Result<Item, SendError<SinkItem>>;

  fn poll_next(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<Self::Item>> {
    match self.receiver.poll_recv(cx) {
      Poll::Ready(Some(data)) => Poll::Ready(Some(Ok(data))),
      Poll::Ready(None) => Poll::Ready(None),
      Poll::Pending => Poll::Pending,
    }
  }

  //fn poll_next(
  //  mut self: std::pin::Pin<&mut Self>,
  //  cx: &mut std::task::Context<'_>,
  //) -> Poll<Option<Self::Item>> {
  //  self.receiver.poll_recv(cx)
  //}

  fn size_hint(&self) -> (usize, Option<usize>) {
    (0, None)
  }
}

impl<Item, SinkItem> Sink<SinkItem> for UnboundedChannel<Item, SinkItem> {
  type Error = SendError<SinkItem>;

  fn poll_ready(
    self: std::pin::Pin<&mut Self>,
    _: &mut std::task::Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn start_send(
    self: std::pin::Pin<&mut Self>,
    item: SinkItem,
  ) -> Result<(), Self::Error> {
    self.sender.send(item)?;
    Ok(())
  }

  fn poll_flush(
    self: std::pin::Pin<&mut Self>,
    _: &mut std::task::Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    Poll::Ready(Ok(()))
  }

  fn poll_close(
    mut self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Result<(), Self::Error>> {
    match self.as_mut().as_mut().poll_flush(cx) {
      Poll::Pending => return Poll::Pending,
      Poll::Ready(val) => val?,
    };

    self.receiver.close();

    Poll::Ready(Ok(()))
  }
}
