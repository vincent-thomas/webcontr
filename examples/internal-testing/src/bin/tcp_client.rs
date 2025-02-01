use std::time::Duration;

use internal_testing::{
  HelloCommand, PingCommand, PingCommandClient, PingCommandRequest,
  PingCommandResponse,
};
use tokio::{
  io::{AsyncReadExt, AsyncWriteExt},
  net::{TcpListener, TcpStream},
  task,
};

use webcontr::prelude::*;

struct PingServer;

impl PingCommand for PingServer {
  async fn hello(&self, value: String, value1: String) -> String {
    "World".to_string()
  }

  async fn hello2(&self, value: u8) -> String {
    todo!()
  }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut result = TcpStream::connect("0.0.0.0:4000").await.unwrap();

  let req = PingCommandRequest::hello {
    value1: "testing".into(),
    value: "verynice".into(),
  };

  let bytes = bincode::serialize(&req)?;

  result.write_i32_le(bytes.len() as i32).await?;
  result.write_all(&bytes).await?;

  let buffer_len = result.read_i32_le().await?;
  let mut buffer = vec![0; buffer_len as usize];
  result.read_exact(&mut buffer).await?;

  let response: PingCommandResponse = bincode::deserialize(&buffer).unwrap();
  println!("{:#?}", response);

  Ok(())
}
