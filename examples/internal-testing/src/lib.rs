//use futures_util::{sink::SinkExt, stream::Stream, stream::StreamExt};
//use tokio::task;
use webcontr::{transport::Transport, Serve};
/// Very nice
#[webcontr::service]
pub trait PingCommand {
  async fn hello(value: String, value1: String) -> String;
  async fn hello2(value: u8) -> String;
}

/// Very nice
#[webcontr::service]
pub trait HelloCommand {
  async fn ping() -> bool;
}
