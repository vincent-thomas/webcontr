use internal_testing::TestingCommandClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
  let mut client = TestingCommandClient::new("localhost:4000".into());
  let res = client.ping().await?;
  println!("result: {res:?}");

  Ok(())
}
