use frame::{RequestFrameCodec, ResponseFrameCodec};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

/// Frame and Codec
pub mod frame;

pub fn request_transport<T>(io: T) -> Framed<T, RequestFrameCodec>
where
  T: AsyncRead + AsyncWrite + Unpin,
{
  Framed::new(io, RequestFrameCodec)
}

pub fn response_transport<T>(io: T) -> Framed<T, ResponseFrameCodec>
where
  T: AsyncRead + AsyncWrite + Unpin,
{
  Framed::new(io, ResponseFrameCodec)
}

#[cfg(test)]
mod tests {
  use crate::transport::tcp::frame::RequestFrame;
  use bytes::Bytes;
  use futures_util::StreamExt;

  // TODO: Set up test

  //#[tokio::test]
  //async fn reading_and_writing() {
  //  let mut vec = Vec::default();
  //
  //  let orig = Frame::new("test".to_string(), Bytes::from(r"payload"));
  //
  //  write_frame(&mut vec, orig.clone()).await.unwrap();
  //
  //  let mut frame = framed_read(vec.as_slice());
  //
  //  let value = frame.next().await.unwrap().unwrap();
  //  let value1 = frame.next().await;
  //
  //  assert!(value == orig);
  //  assert!(value1.map(|_| ()) == None); // The Some(x) is not PartialEq
  //}
}
