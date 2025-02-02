use internal_testing::{PingCommandClient, TestingCommandClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut client = PingCommandClient::new("localhost:4000".into());

  let res = client.hello("bög".to_string()).await?;
  println!("result: {res:?}");

  let mut client = TestingCommandClient::new("localhost:4000".into());
  let res = client.ping().await?;
  println!("result: {res:?}");

  Ok(())
}
