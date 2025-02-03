use webcontr::{
  prelude::{
    bincode,
    serde::{Deserialize, Serialize},
    Bytes, Service,
  },
  transport::frame::ResponseErrorKind,
};

#[derive(Debug, Deserialize, Serialize)]
#[serde(crate = "::webcontr::prelude::serde")]
pub struct Request {
  pub tewting: String,
}
#[webcontr::service]
pub trait TestingCommand {
  async fn ping() -> bool;
}
