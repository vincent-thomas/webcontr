use bytes::{Bytes, BytesMut};
use criterion::{criterion_group, criterion_main, Criterion};
use tokio_util::codec::{Decoder, Encoder};
use webcontr::transport::frame::{
  RequestFrame, RequestFrameCodec, ResponseFrame, ResponseFrameCodec,
};

fn response() {
  let mut bytes = BytesMut::default();

  let response_frame = ResponseFrame::with_payload(Bytes::from("hello world"));
  ResponseFrameCodec.encode(response_frame.clone(), &mut bytes).unwrap();

  let response2 = ResponseFrameCodec.decode(&mut bytes).unwrap().unwrap();

  assert!(response2 == response_frame)
}

fn request() {
  let mut bytes = BytesMut::default();

  let response_frame =
    RequestFrame::new("cmd".to_string(), Bytes::from("hello world"));
  RequestFrameCodec.encode(response_frame.clone(), &mut bytes).unwrap();

  let response2 = RequestFrameCodec.decode(&mut bytes).unwrap().unwrap();

  assert!(response2 == response_frame)
}

fn bench_fibonacci(c: &mut Criterion) {
  c.bench_function("from response frame codec", |b| b.iter(|| response()));
  c.bench_function("from request frame codec", |b| b.iter(|| request()));
}

criterion_group!(benches, bench_fibonacci);
criterion_main!(benches);
