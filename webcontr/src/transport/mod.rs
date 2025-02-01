pub mod channel;

use std::error::Error;

use futures_util::{Sink, Stream};

pub trait Transport<Item, SinkItem>
where
  Self: Stream<Item = Result<Item, <Self as Sink<SinkItem>>::Error>>,
  Self:
    Sink<SinkItem, Error = <Self as Transport<Item, SinkItem>>::TransportError>,
  <Self as Sink<SinkItem>>::Error: Error,
{
  type TransportError: Error + Send + 'static;
}

impl<T, Item, SinkItem, E> Transport<Item, SinkItem> for T
where
  T: ?Sized,
  T: Stream<Item = Result<Item, E>>,
  T: Sink<SinkItem, Error = E>,
  T::Error: Error + Send + 'static,
{
  type TransportError = E;
}
