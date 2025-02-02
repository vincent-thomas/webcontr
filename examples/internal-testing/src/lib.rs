/// Very nice
#[webcontr::service]
pub trait PingCommand {
  /// testing
  async fn hello(value: String, value1: String) -> String;
  async fn hello2(value: u8) -> String;
}

#[webcontr::service]
pub trait TestingCommand {
  async fn ping() -> bool;
}
