use std::io;

use internal_testing::{PingCommand, TestingCommand};
use tokio::net::TcpListener;
use webcontr::Server;

struct PingServer;

#[webcontr::async_trait]
impl PingCommand for PingServer {
  async fn hello(&self, value: String, value1: String) -> String {
    format!("{value}+{value1}")
  }

  async fn hello2(&self, value: u8) -> String {
    format!("Hello: {}", value)
  }
}
struct TestingServer;

#[webcontr::async_trait]
impl TestingCommand for TestingServer {
  async fn ping(&self) -> bool {
    true
  }
}

#[tokio::main]
async fn main() -> io::Result<()> {
  let server = Server::default()
    .add_service(PingServer.into_serve())
    .add_service(TestingServer.into_serve());

  let tcp = TcpListener::bind("0.0.0.0:4000").await?;

  server.serve(tcp).await
}
