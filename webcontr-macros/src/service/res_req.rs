use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Ident, PatType, ReturnType, Visibility};

#[derive(Debug, Clone)]
pub struct ServiceResponse {
  vis: Visibility,
  pub ident: Ident,
  rpcs: Vec<(Ident, ReturnType)>,
}

impl ServiceResponse {
  pub fn new(
    vis: Visibility,
    name: Ident,
    rpcs: Vec<(Ident, ReturnType)>,
  ) -> Self {
    Self {
      vis,
      ident: Ident::new(&format!("{}Response", name), name.span()),
      rpcs,
    }
  }
}

impl ToTokens for ServiceResponse {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { vis, ident, rpcs } = self;
    let keys = rpcs.iter().map(|(ident, return_type)| {
      let output = match return_type {
        ReturnType::Default => quote! {()},
        ReturnType::Type(_, _type) => {
          quote! { #_type }
        }
      };
      quote! {
          #[allow(non_camel_case_types)]
          #ident(#output)
      }
    });

    tokens.extend(quote! {
        #[allow(no_docs)]
        #[derive(Debug, webcontr::prelude::serde::Deserialize, webcontr::prelude::serde::Serialize)]
        #[serde(crate = "::webcontr::prelude::serde")]
        #vis enum #ident {
            #(#keys),*
        }
    })
  }
}

#[derive(Debug, Clone)]
pub struct ServiceRequest {
  pub ident: Ident,
  pub args: Vec<(Ident, Vec<PatType>)>,
  pub vis: Visibility,
}
impl ServiceRequest {
  pub fn new(
    vis: Visibility,
    name: Ident,
    args: Vec<(Ident, Vec<PatType>)>,
  ) -> Self {
    Self {
      vis,
      ident: Ident::new(&format!("{}Request", name), name.span()),
      args,
    }
  }
}

impl ToTokens for ServiceRequest {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { vis, ident, args } = self;
    let keys = args.iter().map(|(ident, values)| {
      let values_iter = values.iter().map(|_type| {
        let key = _type.pat.clone();
        let value = _type.ty.clone();
        quote! {
            #key: #value
        }
      });

      quote! {
          #[allow(non_camel_case_types)]
          #ident {
            #(#values_iter),*
          }
      }
    });

    tokens.extend(quote! {
        #[allow(no_docs)]
        #[derive(Debug, webcontr::prelude::serde::Deserialize, webcontr::prelude::serde::Serialize)]
        #[serde(crate = "::webcontr::prelude::serde")]
        #vis enum #ident {
            #(#keys),*
        }
    })
  }
}
