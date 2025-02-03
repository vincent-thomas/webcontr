use std::{io, time::Duration};

use internal_testing::TestingCommand;
use tokio::net::TcpListener;
use webcontr::{tls::TLSPaths, Server};

#[derive(Clone)]
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
  let server = Server::default().add_service(TestingServer.into_serve());

  let tcp = TcpListener::bind("0.0.0.0:4000").await?;

  let tls_paths = TLSPaths::from_paths(
    "./webcontr/tests/certs/chain.pem",
    "./webcontr/tests/certs/end.key",
  );

  server.serve(tcp, tls_paths).await
}
