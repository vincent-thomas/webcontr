pub mod prelude;
pub mod transport;

pub use webcontr_macros::service;

pub trait Serve<Req, Res> {
  async fn serve(&self, req: Req) -> Res;
}
