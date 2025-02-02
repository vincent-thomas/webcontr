use std::{io, time::Duration};

use internal_testing::{PingCommand, Request, TestingCommand};
use tokio::net::TcpListener;
use webcontr::Server;

struct PingServer;

#[webcontr::async_trait]
impl PingCommand for PingServer {
  async fn hello(&self, _: String) -> Vec<String> {
    tokio::time::sleep(Duration::from_secs(4)).await;
    Vec::from_iter(["very nice".into(), "arry".into()])
  }

  async fn hello2(&self, req: Request) -> String {
    req.tewting
  }
}
struct TestingServer;

#[webcontr::async_trait]
impl TestingCommand for TestingServer {
  async fn ping(&self) -> bool {
    tokio::time::sleep(Duration::from_secs(4)).await;
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
