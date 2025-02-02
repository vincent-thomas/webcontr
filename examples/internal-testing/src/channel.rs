use internal_testing::{HelloCommand, PingCommand, PingCommandClient};
use tokio::task;

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
async fn main() {
  let (client, mut server) = webcontr::transport::channel::unbounded();
  let ping = PingServer.into_serve();

  let server_thread = task::spawn(async move {
    while let Some(payload) = server.next().await {
      let res = ping.serve(payload.unwrap()).await;
      let _ = server.send(res).await;
    }
  });

  let mut client = PingCommandClient::new(client);

  let response =
    client.hello("testing".to_string(), "testing".to_string()).await.unwrap();

  let res =
    client.hello("testing".to_string(), "tesing".to_string()).await.unwrap();

  println!("response: {}", response);

  tokio::join!(server_thread).0.unwrap();
}
