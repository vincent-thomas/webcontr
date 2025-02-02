use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite};

use super::frame::{RequestFrameCodec, ResponseFrameCodec};

pub fn request_transport<T>(io: T) -> FramedRead<T, RequestFrameCodec>
where
  T: AsyncRead,
{
  FramedRead::new(io, RequestFrameCodec)
}

pub fn response_transport<T>(io: T) -> FramedWrite<T, ResponseFrameCodec>
where
  T: AsyncWrite,
{
  FramedWrite::new(io, ResponseFrameCodec)
}

pub mod client {
  use tokio::io::{AsyncRead, AsyncWrite};
  use tokio_util::codec::{FramedRead, FramedWrite};

  use crate::transport::frame::{RequestFrameCodec, ResponseFrameCodec};

  pub fn request_transport<T>(io: T) -> FramedWrite<T, RequestFrameCodec>
  where
    T: AsyncWrite,
  {
    FramedWrite::new(io, RequestFrameCodec)
  }

  pub fn response_transport<T>(io: T) -> FramedRead<T, ResponseFrameCodec>
  where
    T: AsyncRead,
  {
    FramedRead::new(io, ResponseFrameCodec)
  }
}

#[cfg(test)]
mod tests {
  use crate::transport::frame::{
    RequestFrame, RequestFrameCodec, ResponseFrame, ResponseFrameCodec,
  };

  use bytes::Bytes;
  use futures_util::{SinkExt, StreamExt};
  use tokio::io::duplex;
  use tokio_util::codec::{FramedRead, FramedWrite};
  // Import the functions under test.
  use super::{
    client::{
      request_transport as client_request_transport,
      response_transport as client_response_transport,
    },
    request_transport as server_request_transport,
    response_transport as server_response_transport,
  };

  // These tests assume that RequestFrameCodec and ResponseFrameCodec encode/decode
  // a String. Adjust the test values and types if your codecs work with a different type.

  #[tokio::test]
  async fn test_server_request_transport() {
    // Simulate a duplex connection.
    let (client_io, server_io) = duplex(64);
    // Server uses a FramedRead to receive requests.
    let mut server_reader = server_request_transport(server_io);
    // Client uses a FramedWrite with the same codec to send a request.
    let mut client_writer = FramedWrite::new(client_io, RequestFrameCodec);

    let request = RequestFrame::new("cmd".into(), Bytes::from("payload"));
    client_writer.send(request.clone()).await.unwrap();

    // Server should decode the same request.
    let received =
      server_reader.next().await.expect("No message received").unwrap();
    assert_eq!(received, request);
  }

  #[tokio::test]
  async fn test_server_response_transport() {
    let (server_io, client_io) = tokio::io::duplex(64);
    let mut server_writer = server_response_transport(server_io);
    let mut client_reader =
      tokio_util::codec::FramedRead::new(client_io, ResponseFrameCodec);

    let request = ResponseFrame::Payload(Bytes::from("hello"));

    server_writer.send(request.clone()).await.unwrap();

    let received =
      client_reader.next().await.expect("No message received").unwrap();
    assert_eq!(received, request);
  }

  #[tokio::test]
  async fn test_client_request_transport() {
    // Create a duplex connection.
    let (server_io, client_io) = duplex(64);
    // Client uses a FramedWrite to send requests.
    let mut client_writer = client_request_transport(client_io);
    // Server manually sets up a FramedRead with the same codec to receive requests.
    let mut server_reader = FramedRead::new(server_io, RequestFrameCodec);

    let request =
      RequestFrame::new("command".to_string(), Bytes::from("payload"));
    client_writer.send(request.clone()).await.unwrap();

    let received =
      server_reader.next().await.expect("No message received").unwrap();
    assert_eq!(received, request);
  }

  #[tokio::test]
  async fn test_client_response_transport() {
    // Create a duplex connection.
    let (client_io, server_io) = duplex(64);
    // Server uses a FramedWrite to send responses.
    let mut server_writer = FramedWrite::new(server_io, ResponseFrameCodec);
    // Client uses a FramedRead to receive responses.
    let mut client_reader = client_response_transport(client_io);

    let response = ResponseFrame::Payload(Bytes::from("hello"));
    server_writer.send(response.clone()).await.unwrap();

    let received =
      client_reader.next().await.expect("No message received").unwrap();
    assert_eq!(received, response);
  }
}
