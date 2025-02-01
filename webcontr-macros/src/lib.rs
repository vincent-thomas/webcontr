mod service;

use std::{iter::Map, slice::Iter};

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use service::{
  res_req::{ServiceRequest, ServiceResponse},
  Service,
};
use syn::{
  braced, parenthesized, parse::Parse, parse_macro_input, spanned::Spanned,
  Attribute, FnArg, Ident, Pat, PatType, ReturnType, Token, Visibility,
};

#[derive(Debug)]
struct Rpc {
  attrs: Vec<Attribute>,
  ident: Ident,
  args: Vec<PatType>,
  output: ReturnType,
}

impl ToTokens for Rpc {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    let Self { attrs, ident, args, output } = self;

    let args = args.iter().map(|pat| {
      let fn_arg = FnArg::Typed(pat.clone());
      fn_arg
    });

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
        FnArg::Receiver(_) => {
          unimplemented!("'self' in rpc methods is not supported")
        }
        FnArg::Typed(arg) => {
          if let Pat::Ident(_) = arg.pat.as_ref() {
            parsed_params.push(arg.clone());
          } else {
            return Err(syn::Error::new(
              Span::from(arg.span()),
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

#[proc_macro_attribute]
pub fn service(_: TokenStream, input: TokenStream) -> TokenStream {
  let input = parse_macro_input!(input as Service);

  impl_service(input).into()
}

fn impl_service(input: Service) -> TokenStream2 {
  ServiceGenerator::new(input).into_token_stream()
}

struct ServiceGenerator {
  service_request: ServiceRequest,
  service_response: ServiceResponse,
  service: Service,
}

impl ServiceGenerator {
  fn new(service: Service) -> Self {
    let req_args = service
      .rpcs
      .iter()
      .map(|rpc| (rpc.ident.clone(), rpc.args.clone()))
      .collect();

    let res_args = service
      .rpcs
      .iter()
      .map(|rpc| (rpc.ident.clone(), rpc.output.clone()))
      .collect();

    ServiceGenerator {
      service_request: ServiceRequest::new(service.ident.clone(), req_args),
      service_response: ServiceResponse::new(service.ident.clone(), res_args),
      service,
    }
  }
  fn trait_service(&self) -> TokenStream2 {
    let Service { attrs, vis, ident, rpcs } = &self.service;

    let rpcs_iter = rpcs.iter();
    let attrs_iter = attrs.iter();

    let rpcs_args: Vec<Vec<&Pat>> = rpcs_iter
      .clone()
      .map(|rpc| rpc.args.iter().map(|arg| &*arg.pat).collect::<Vec<&Pat>>())
      .collect();

    let req_ident = &self.service_request.ident;
    let res_ident = &self.service_response.ident;

    // Same as function names
    let req_variants =
      self.service_request.args.iter().map(|variant| variant.0.clone());
    let res_ident = &self.service_response.ident;

    let serve_struct_ident = Ident::new(
      &format!("{}Serve", self.service.ident.to_string()),
      self.service.ident.span(),
    );

    quote! {
       #(#attrs_iter),*
       #vis trait #ident: Sized {
           #(#rpcs_iter)*
           fn service_name() -> &'static str {
               stringify!(#ident)
           }
           fn into_serve(self) -> #serve_struct_ident<Self> {
               #serve_struct_ident {
                   service: self
               }
           }
       }
    }
  }
  fn serve_fn(&self) -> TokenStream2 {
    let Service { attrs, vis, ident, rpcs } = &self.service;

    let rpcs_args: Vec<Vec<&Pat>> = rpcs
      .iter()
      .clone()
      .map(|rpc| rpc.args.iter().map(|arg| &*arg.pat).collect::<Vec<&Pat>>())
      .collect();

    let req_ident = &self.service_request.ident;

    // Same as function names
    let variants =
      self.service_request.args.iter().map(|variant| variant.0.clone());
    let res_ident = &self.service_response.ident;

    let serve_struct_ident = Ident::new(
      &format!("{}Serve", self.service.ident.to_string()),
      self.service.ident.span(),
    );
    quote! {
        struct #serve_struct_ident<S> {
            service: S
        }

        impl<A: #ident> webcontr::Serve<#req_ident, #res_ident> for #serve_struct_ident<A> {
           async fn serve(&self, req: #req_ident) -> #res_ident {
               match req {
                   #(
                    #req_ident::#variants { #(#rpcs_args),* } => {
                        let out = #ident::#variants(&self.service, #(#rpcs_args),*).await;
                        #res_ident::#variants(out)
                    }
                    ),*
               }
           }
        }
    }
  }

  fn service_response(&self) -> TokenStream2 {
    self.service_response.to_token_stream()
  }
  fn service_request(&self) -> TokenStream2 {
    self.service_request.to_token_stream()
  }
  fn service_client(&self) -> TokenStream2 {
    let ident = Ident::new(
      &format!("{}Client", self.service.ident.to_string()),
      self.service.ident.span(),
    );

    let rpc_ident = self.service.rpcs.iter().map(|rpc| rpc.ident.clone());

    let rpc_args_types: Vec<Vec<PatType>> =
      self.service.rpcs.iter().map(|rpc| rpc.args.clone()).collect();
    let rpc_args: Vec<Vec<&Pat>> = self
      .service
      .rpcs
      .iter()
      .map(|rpc| rpc.args.iter().map(|argtype| &*argtype.pat).collect())
      .collect();

    let rpc_return_type: Vec<TokenStream2> = self
      .service
      .rpcs
      .iter()
      .map(|rpc| match &rpc.output {
        ReturnType::Default => quote! {()},
        ReturnType::Type(_, _type) => quote! {#_type},
      })
      .collect();

    let rpc_res_ident = &self.service_response.ident;
    let rpc_req_ident = &self.service_request.ident;

    quote! {
        pub struct #ident<T> {
            transport: T
        }

        impl<T> #ident<T> where T: webcontr::transport::Transport<#rpc_res_ident, #rpc_req_ident> + Unpin {
            pub fn new(transport: T) -> Self {
                Self { transport }
            }

            #(
                pub async fn #rpc_ident(&mut self, #(#rpc_args_types),*) -> Result<#rpc_return_type, ()> {
                    let req = #rpc_req_ident::#rpc_ident { #(#rpc_args),* };
                    self.transport.feed(req).await;
                    self.transport.flush().await;

                    let response = match self.transport.next().await {
                        Some(payload) => match payload.map_err(|_| ())? {
                            #rpc_res_ident::#rpc_ident(response) => response,
                            _ => unreachable!()
                        },
                        None => return Err(())
                    };
                    Ok(response)
                }
            )*
        }
    }
    //impl<T> PingClient<T>
    //where
    //  T: Transport<PingCommandResponse, PingCommandRequest> + Unpin,
    //{
    //  fn new(transport: T) -> Self {
    //    Self { transport }
    //  }
    //
    //  pub async fn ping(
    //    &mut self,
    //    value: String,
    //    value1: String,
    //  ) -> Result<String, ()> {
    //    let req = PingCommandRequest::ping { value, value1 };
    //    self.transport.feed(req).await;
    //    self.transport.flush().await;
    //
    //    let response = match self.transport.next().await {
    //      Some(payload) => match payload.map_err(|_| ())? {
    //        PingCommandResponse::ping(response) => response,
    //        _ => unreachable!(),
    //      },
    //      None => return Err(()),
    //    };
    //
    //    Ok(response)
    //  }
    //}
  }
}

impl ToTokens for ServiceGenerator {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    tokens.extend([
      self.trait_service(),
      self.serve_fn(),
      self.service_request(),
      self.service_response(),
      self.service_client(),
    ]);
  }
}
