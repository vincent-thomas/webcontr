use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
  parenthesized, parse::Parse, spanned::Spanned, Attribute, FnArg, Ident, Pat,
  PatType, ReturnType, Token,
};

#[derive(Debug)]
pub struct Rpc {
  pub attrs: Vec<Attribute>,
  pub ident: Ident,
  pub args: Vec<PatType>,
  pub output: ReturnType,
}

impl ToTokens for Rpc {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { attrs, ident, args, output } = self;

    let args = args.iter().map(|pat| FnArg::Typed(pat.clone()));

    let attrs_iter = attrs.iter();

    let out = quote! {
        #(#attrs_iter),*
        async fn #ident(&self, #(#args),* ) #output;
    };

    tokens.extend(out);
  }
}

impl Parse for Rpc {
  fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
    let attrs = input.call(Attribute::parse_outer)?;
    let _async = input.parse::<Token![async]>()?;
    let _fn = input.parse::<Token![fn]>()?;

    let ident = input.parse::<Ident>()?;

    let params;
    parenthesized!(params in input);

    let mut parsed_params = Vec::default();
    for item in &params.parse_terminated(FnArg::parse, Token![,])? {
      match item {
        FnArg::Receiver(value) => {
          return Err(syn::Error::new(
            value.span(),
            "'self' in rpc methods are not needed",
          ))
        }

        FnArg::Typed(arg) => {
          if let Pat::Ident(_) = arg.pat.as_ref() {
            parsed_params.push(arg.clone());
          } else {
            return Err(syn::Error::new(
              arg.span(),
              "Patterns are not supported inside a rpc method",
            ));
          }
        }
      };
    }

    let output = input.parse()?;

    input.parse::<Token![;]>()?;

    Ok(Rpc { attrs, ident, args: parsed_params, output })
  }
}
