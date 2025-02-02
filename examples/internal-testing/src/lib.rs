use webcontr::prelude::serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "::webcontr::prelude::serde")]
pub struct Request {
  pub tewting: String,
}

/// Very nice
#[webcontr::service]
pub trait PingCommand {
  /// testing
  async fn hello(nice: String) -> Vec<String>;
  async fn hello2(req: Request) -> String;
}

#[webcontr::service]
pub trait TestingCommand {
  async fn ping() -> bool;
}
