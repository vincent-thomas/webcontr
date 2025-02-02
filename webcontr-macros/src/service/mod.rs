use rpc::Rpc;
use syn::{braced, parse::Parse, Attribute, Ident, Token, Visibility};

pub mod res_req;
pub mod rpc;

#[derive(Debug)]
pub struct Service {
  pub attrs: Vec<Attribute>,
  pub vis: Visibility,
  pub ident: Ident,
  pub rpcs: Vec<Rpc>,
}

impl Parse for Service {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = input.call(Attribute::parse_outer)?;
    let vis = input.parse::<Visibility>()?;
    input.parse::<Token![trait]>()?;
    let ident = input.parse::<Ident>()?;

    let content;

    braced!(content in input);

    let mut rpcs = Vec::default();

    while !content.is_empty() {
      rpcs.push(content.parse::<Rpc>()?);
    }

    Ok(Service { attrs, vis, ident, rpcs })
  }
}
