use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::Framed;

use super::frame::{RequestFrameCodec, ResponseFrameCodec};

pub fn request_transport<T>(io: T) -> Framed<T, RequestFrameCodec>
where
  T: AsyncRead + AsyncWrite,
{
  Framed::new(io, RequestFrameCodec)
}

pub fn response_transport<T>(io: T) -> Framed<T, ResponseFrameCodec>
where
  T: AsyncWrite + AsyncRead,
{
  Framed::new(io, ResponseFrameCodec)
}

pub mod client {
  use super::{request_transport, response_transport};
  use bytes::Bytes;
  use futures_util::{SinkExt, StreamExt};
  use serde::{de::DeserializeOwned, Serialize};
  use tokio::net::TcpStream;

  use crate::{
    transport::frame::{RequestFrame, ResponseFrame},
    ClientError,
  };

  pub async fn send_client_req<Req: Serialize, Res: DeserializeOwned>(
    cmd: &'static str,
    req: Req,
    addr: &str,
  ) -> Result<Res, ClientError> {
    let stream =
      TcpStream::connect(addr).await.map_err(ClientError::IoError)?;
    let stream = {
      #[cfg(not(feature = "tls"))]
      {
        stream
      }
      #[cfg(feature = "tls")]
      {
        use std::sync::Arc;
        use tokio_rustls::{
          rustls::{
            pki_types::{pem::PemObject, CertificateDer, ServerName},
            ClientConfig, RootCertStore,
          },
          TlsConnector,
        };

        const ROOT: &str = include_str!("../../../tests/certs/root.pem");
        let mut client_root_cert_store = RootCertStore::empty();
        for root in CertificateDer::pem_slice_iter(ROOT.as_bytes()) {
          client_root_cert_store.add(root.unwrap()).unwrap();
        }

        let cconfig = ClientConfig::builder()
          .with_root_certificates(client_root_cert_store)
          .with_no_client_auth();
        let connector = TlsConnector::from(Arc::new(cconfig));
        connector
          .connect(
            ServerName::try_from(addr.split(":").collect::<Vec<&str>>()[0])
              .unwrap()
              .to_owned(),
            stream,
          )
          .await
          .unwrap()
      }
    };

    let body = bincode::serialize(&req).map_err(ClientError::EncodingError)?;

    let request_frame = RequestFrame::new(cmd.to_string(), Bytes::from(body));
    let mut transport = request_transport(stream);
    SinkExt::send(&mut transport, request_frame).await.unwrap();

    let stream = transport.into_inner();

    let mut transport = response_transport(stream);

    let response_frame_outer;

    loop {
      let response_frame_in = StreamExt::next(&mut transport).await;
      match response_frame_in {
        None => continue,
        Some(v) => {
          response_frame_outer = v.map_err(ClientError::IoError)?;
          break;
        }
      }
    }

    let thing: Res = match response_frame_outer {
      ResponseFrame::Error(err) => return Err(ClientError::ServerError(err)),
      ResponseFrame::Payload(data) => bincode::deserialize(&data).unwrap(),
    };

    Ok(thing)
  }
}

#[cfg(test)]
mod tests {
  use crate::transport::{
    frame::{
      RequestFrame, RequestFrameCodec, ResponseFrame, ResponseFrameCodec,
    },
    tcp::{request_transport, response_transport},
  };

  use bytes::Bytes;
  use futures_util::{SinkExt, StreamExt};
  use tokio::io::duplex;
  use tokio_util::codec::{FramedRead, FramedWrite};

  // These tests assume that RequestFrameCodec and ResponseFrameCodec encode/decode
  // a String. Adjust the test values and types if your codecs work with a different type.

  #[tokio::test]
  async fn test_server_request_transport() {
    // Simulate a duplex connection.
    let (client_io, server_io) = duplex(64);
    // Server uses a FramedRead to receive requests.
    let mut server_reader = request_transport(server_io);
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
    let mut server_writer = response_transport(server_io);
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
    let mut client_writer = request_transport(client_io);
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
    let mut client_reader = response_transport(client_io);

    let response = ResponseFrame::Payload(Bytes::from("hello"));
    server_writer.send(response.clone()).await.unwrap();

    let received =
      client_reader.next().await.expect("No message received").unwrap();
    assert_eq!(received, response);
  }
}
