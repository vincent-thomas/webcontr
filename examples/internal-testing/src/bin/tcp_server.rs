use std::{collections::HashMap, time::Duration};

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
  let ping = PingServer.into_serve();

  let test = TcpListener::bind("0.0.0.0:4000").await?;
  println!("ready");
  loop {
    let (mut stream, _) = test.accept().await?;

    let payload_size = stream.read_i32_le().await?;

    let mut buffer = vec![0; payload_size as usize];
    stream.read_exact(&mut buffer).await?;

    let request: PingCommandRequest = bincode::deserialize(&buffer).unwrap();
    let response = ping.serve(request).await;

    let response_bytes = bincode::serialize(&response)?;
    stream.write_i32_le(response_bytes.len() as i32).await?;
    stream.write_all(&response_bytes).await?;
  }
}

#[derive(Default)]
pub struct TcpServer {
  hash: HashMap<String, Box<dyn Serve>>,
}

impl TcpServer {
  async fn run(mut self, tcp: TcpListener) {}
}

//pub struct TcpClient(TcpStream);
