use futures_util::{SinkExt, StreamExt};
use internal_testing::{PingCommand, PingCommandClient};
use tokio::task;

struct PingServer;

impl PingCommand for PingServer {
  async fn hello(&self, value: String, value1: String) -> String {
    "World".to_string()
  }
}

#[tokio::main]
async fn main() {
  let (client, mut server) = webcontr::transport::channel::unbounded();
  let ping = PingServer;

  let service = ping.into_serve();
  let server_thread = task::spawn(async move {
    while let Some(payload) = server.next().await {
      let res = service.serve(payload.unwrap()).await;
      let _ = server.send(res).await;
    }
  });

  let mut client = PingCommandClient::new(client);

  let response =
    client.hello("testing".to_string(), "testing".to_string()).await.unwrap();

  println!("response: {}", response);

  tokio::join!(server_thread);
}
