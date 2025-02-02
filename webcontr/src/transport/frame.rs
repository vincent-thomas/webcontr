use std::io::{self, ErrorKind};

use bytes::{Buf, BufMut, Bytes, BytesMut};
use tokio_util::codec::{Decoder, Encoder};

use thiserror::Error;

#[derive(Debug, PartialEq, Clone)]
pub enum ResponseFrame {
  Payload(Bytes),
  Error(ResponseErrorKind),
}
#[derive(Error, Debug, PartialEq, Clone)]
pub enum ResponseErrorKind {
  #[error("method not found")]
  MethodNotFound, // 1
  #[error("invalid request")]
  InvalidRequest, // 2
}

impl ResponseFrame {
  pub fn with_payload(response: Bytes) -> Self {
    Self::Payload(response)
  }
}

/// Codec For [crate::transport::tcp::frame::ResponseFrame]
pub struct ResponseFrameCodec;

impl Decoder for ResponseFrameCodec {
  type Item = ResponseFrame;
  type Error = io::Error;

  fn decode(
    &mut self,
    src: &mut BytesMut,
  ) -> Result<Option<Self::Item>, Self::Error> {
    if src.len() < 1 {
      return Ok(None); // Not enough data for command length
    }

    let mut buf = src.clone();
    match buf.get_u8() {
      // Scenario 0: Normal request with a payload.
      0 => {
        if src.len() < 2 {
          return Ok(None); // Not enough data for command length
        }

        let response_len = buf.get_u16() as usize;

        if src.len() < response_len {
          return Ok(None);
        }

        let response_bytes = buf.split_to(response_len);
        src.advance(response_len);

        Ok(Some(ResponseFrame::with_payload(response_bytes.freeze())))
      }
      // Scenario 1: If client send invalid rpc method.
      1 => Ok(Some(ResponseFrame::Error(ResponseErrorKind::MethodNotFound))),
      // Scenario 2: Totally unreadable/invalid request.
      2 => Ok(Some(ResponseFrame::Error(ResponseErrorKind::InvalidRequest))),
      _ => Err(io::Error::new(ErrorKind::InvalidData, "Invalid first byte")),
    }
  }
}

impl Encoder<ResponseFrame> for ResponseFrameCodec {
  type Error = io::Error;

  fn encode(
    &mut self,
    frame: ResponseFrame,
    dst: &mut BytesMut,
  ) -> Result<(), Self::Error> {
    match frame {
      ResponseFrame::Error(err) => match err {
        ResponseErrorKind::MethodNotFound => dst.put_u8(1),
        ResponseErrorKind::InvalidRequest => dst.put_u8(2),
      },
      ResponseFrame::Payload(payload) => {
        dst.put_u8(0);
        dst.put_u16(payload.len() as u16);
        dst.extend_from_slice(&payload);
      }
    };

    Ok(())
  }
}

#[derive(Debug, PartialEq, Clone)]
pub struct RequestFrame {
  pub command: String,
  pub arguments: Bytes,
}

impl RequestFrame {
  pub fn new(cmd: String, payload: Bytes) -> Self {
    Self { command: cmd, arguments: payload }
  }

  //pub fn args<'a, R: Deserialize<'a>>(&'a mut self) -> bincode::Result<R> {
  //  bincode::deserialize(&self.arguments)
  //}
}

pub struct RequestFrameCodec;

impl Decoder for RequestFrameCodec {
  type Item = RequestFrame;
  type Error = io::Error;

  fn decode(
    &mut self,
    src: &mut BytesMut,
  ) -> Result<Option<RequestFrame>, Self::Error> {
    if src.len() < 4 {
      return Ok(None); // Not enough data for command length
    }

    let mut buf = src.clone();
    let cmd_len = buf.get_u16() as usize;

    if buf.len() < cmd_len + 2 {
      return Ok(None); // Not enough data for full command
    }

    let cmd_bytes = buf.split_to(cmd_len);
    let command = String::from_utf8(cmd_bytes.to_vec()).map_err(|_| {
      io::Error::new(ErrorKind::InvalidData, "Invalid UTF-8 command")
    })?;

    if buf.len() < 2 {
      return Ok(None); // Not enough data for payload length
    }

    let payload_len = buf.get_u16() as usize;

    if buf.len() < payload_len {
      return Ok(None); // Not enough data for full payload
    }

    let payload = buf.split_to(payload_len).freeze();
    src.advance(4 + cmd_len + payload_len);

    Ok(Some(RequestFrame { command, arguments: payload }))
  }
}

impl Encoder<RequestFrame> for RequestFrameCodec {
  type Error = io::Error;

  fn encode(
    &mut self,
    frame: RequestFrame,
    dst: &mut BytesMut,
  ) -> Result<(), Self::Error> {
    let cmd_bytes = frame.command.as_bytes();
    let cmd_len = cmd_bytes.len();

    dst.put_u16(cmd_len as u16);
    dst.extend_from_slice(cmd_bytes);

    let payload_len = frame.arguments.len();
    dst.put_u16(payload_len as u16);
    dst.extend_from_slice(&frame.arguments);

    Ok(())
  }
}

#[test]
pub fn request_decoding() {
  let mut buffer_vec = Vec::default();

  buffer_vec.extend(5u16.to_be_bytes());
  buffer_vec.extend(b"hello");
  buffer_vec.extend(4u16.to_be_bytes());
  buffer_vec.extend(b"data");

  let mut buffer_mut = BytesMut::from(buffer_vec.as_slice());
  let result = RequestFrameCodec.decode(&mut buffer_mut);

  assert_eq!(
    result.unwrap().unwrap(),
    RequestFrame { command: "hello".into(), arguments: Bytes::from("data") }
  );
}

#[test]
pub fn request_encoding() {
  let frame =
    RequestFrame { command: "hello".into(), arguments: Bytes::from("data") };

  let mut bytes = BytesMut::default();

  RequestFrameCodec.encode(frame, &mut bytes).unwrap();

  let mut buffer_vec = Vec::default();

  buffer_vec.extend(5u16.to_be_bytes());
  buffer_vec.extend(b"hello");
  buffer_vec.extend(4u16.to_be_bytes());
  buffer_vec.extend(b"data");

  assert_eq!(bytes, BytesMut::from(buffer_vec.as_slice()));
}

#[test]
pub fn response_decoding() {
  let mut buffer_vec = Vec::default();

  buffer_vec.extend(0u8.to_be_bytes());
  buffer_vec.extend(4u16.to_be_bytes());
  buffer_vec.extend(b"data");

  let mut buffer_mut = BytesMut::from(buffer_vec.as_slice());
  let result = ResponseFrameCodec.decode(&mut buffer_mut);

  assert_eq!(
    result.unwrap().unwrap(),
    ResponseFrame::with_payload(Bytes::from("data"))
  );

  let mut buffer_vec = Vec::default();

  buffer_vec.extend(1u8.to_be_bytes());

  let mut buffer_mut = BytesMut::from(buffer_vec.as_slice());
  let result = ResponseFrameCodec.decode(&mut buffer_mut);

  assert_eq!(
    result.unwrap().unwrap(),
    ResponseFrame::Error(ResponseErrorKind::MethodNotFound)
  );
}

#[test]
pub fn response_encoding() {
  let frame = ResponseFrame::Error(ResponseErrorKind::MethodNotFound);

  let mut bytes = BytesMut::default();

  ResponseFrameCodec.encode(frame, &mut bytes).unwrap();

  let mut buffer_vec = Vec::default();

  buffer_vec.push(1u8);

  assert_eq!(bytes, BytesMut::from(buffer_vec.as_slice()));

  let data2 = "helloverynice";

  let frame2 = ResponseFrame::with_payload(Bytes::from(data2));

  let mut bytes = BytesMut::default();

  ResponseFrameCodec.encode(frame2, &mut bytes).unwrap();

  let mut buffer_vec = Vec::default();

  buffer_vec.push(0u8);
  buffer_vec.extend((data2.len() as u16).to_be_bytes());
  buffer_vec.extend(data2.as_bytes());

  assert_eq!(bytes, BytesMut::from(buffer_vec.as_slice()));
}
