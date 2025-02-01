use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{Ident, PatType, ReturnType};

#[derive(Debug, Clone)]
pub struct ServiceResponse {
  pub ident: Ident,
  rpcs: Vec<(Ident, ReturnType)>,
}

impl ServiceResponse {
  pub fn new(name: Ident, rpcs: Vec<(Ident, ReturnType)>) -> Self {
    Self {
      ident: Ident::new(&format!("{}Response", name.to_string()), name.span()),
      rpcs,
    }
  }
}

impl ToTokens for ServiceResponse {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { ident, rpcs } = self;
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
        #[derive(Debug)]
        enum #ident {
            #(#keys),*
        }
    })
  }
}

#[derive(Debug, Clone)]
pub struct ServiceRequest {
  pub ident: Ident,
  pub args: Vec<(Ident, Vec<PatType>)>,
}
impl ServiceRequest {
  pub fn new(name: Ident, args: Vec<(Ident, Vec<PatType>)>) -> Self {
    Self {
      ident: Ident::new(&format!("{}Request", name.to_string()), name.span()),
      args,
    }
  }
}

impl ToTokens for ServiceRequest {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { ident, args } = self;
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
        enum #ident {
            #(#keys),*
        }
    })
  }
}
