use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{braced, parse::Parse, Attribute, Ident, Token, Visibility};

use crate::Rpc;

pub mod res_req;

#[derive(Debug)]
pub struct Service {
  pub attrs: Vec<Attribute>,
  pub vis: Visibility,
  pub ident: Ident,
  pub rpcs: Vec<Rpc>,
}

//impl ToTokens for Service {
//  fn to_tokens(&self, tokens: &mut TokenStream2) {
//    let Self { attrs, vis, ident, rpcs } = self;
//
//    let rpcs_iter = rpcs.iter();
//    let attrs_iter = attrs.iter();
//    let out = quote! {
//       #(#attrs_iter),*
//       #vis trait #ident {
//           #(#rpcs_iter)*
//       }
//    };
//    tokens.extend(out);
//  }
//}

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
