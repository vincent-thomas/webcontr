mod service;

use proc_macro::TokenStream;
use proc_macro2::{Span, TokenStream as TokenStream2};
use quote::{quote, ToTokens};
use service::{
  res_req::{ServiceRequest, ServiceResponse},
  Service,
};
use syn::{
  parenthesized, parse::Parse, parse_macro_input, spanned::Spanned, Attribute,
  FnArg, Ident, Pat, PatType, ReturnType, Token,
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

  ServiceGenerator::new(input).into_token_stream().into()
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
      service_request: ServiceRequest::new(
        service.vis.clone(),
        service.ident.clone(),
        req_args,
      ),
      service_response: ServiceResponse::new(
        service.vis.clone(),
        service.ident.clone(),
        res_args,
      ),
      service,
    }
  }
  fn trait_service(&self) -> TokenStream2 {
    let Service { attrs, vis, ident, rpcs } = &self.service;

    let rpcs_iter = rpcs.iter();
    let attrs_iter = attrs.iter();

    let serve_struct_ident = Ident::new(
      &format!("{}Serve", self.service.ident.to_string()),
      self.service.ident.span(),
    );

    quote! {
       #(#attrs_iter),*
       #[webcontr::async_trait]
       #vis trait #ident: Sized {
           #(#rpcs_iter)*

           fn into_serve(self) -> #serve_struct_ident<Self> {
               #serve_struct_ident {
                   service: self
               }
           }
       }
    }
  }
  fn impl_serve_fn(&self) -> TokenStream2 {
    let Service { attrs: _, vis, ident, rpcs } = &self.service;

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
        #vis struct #serve_struct_ident<S> {
            service: S
        }

        #[webcontr::async_trait]
        impl<A: #ident + Send + Sync> webcontr::Serve for #serve_struct_ident<A> {
           async fn serve(&self, req: webcontr::prelude::Bytes) -> Result<webcontr::prelude::Bytes, webcontr::ServeError> {
               let req: #req_ident = webcontr::prelude::bincode::deserialize(&req).map_err(|_| webcontr::ServeError::InvalidRequest)?;
               match req {
                   #(
                    #req_ident::#variants { #(#rpcs_args),* } => {
                        let out = #ident::#variants(&self.service, #(#rpcs_args),*).await;
                        let bytes = webcontr::prelude::bincode::serialize(&#res_ident::#variants(out)).unwrap();
                        Ok(webcontr::prelude::Bytes::from(bytes))
                    }
                    ),*
               }
           }
        }
    }
  }

  fn impl_servicename_fn(&self) -> TokenStream2 {
    let ident = &self.service.ident;
    let serve_struct_ident = Ident::new(
      &format!("{}Serve", self.service.ident.to_string()),
      self.service.ident.span(),
    );

    quote! {
        impl<B> webcontr::ServiceName for #serve_struct_ident<B> {
            fn name(&self) -> &'static str {
                stringify!(#ident)
            }
        }
    }
  }

  fn service_client(&self) -> TokenStream2 {
    let ident = &self.service.ident;
    let client_ident = Ident::new(
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
          pub struct #client_ident {
              addr: String
          }

          impl #client_ident {
              pub fn new(addr: String) -> Self {
                  Self { addr }
              }

              #(
                  pub async fn #rpc_ident(&mut self, #(#rpc_args_types),*) -> Result<#rpc_return_type, webcontr::transport::tcp::frame::ResponseErrorKind> {
                      let stream = webcontr::prelude::TcpStream::connect(&self.addr).await.unwrap();
                      let mut transport = webcontr::transport::tcp::request_transport(stream);

                      let req = #rpc_req_ident::#rpc_ident { #(#rpc_args),* };
                      let body = webcontr::prelude::bincode::serialize(&req).unwrap();

                      let request_frame = webcontr::transport::tcp::frame::RequestFrame::new(
                          stringify!(#ident).to_string(),
                          webcontr::prelude::Bytes::from(body)
                      );

                      webcontr::prelude::SinkExt::send(&mut transport, request_frame).await.unwrap();

                      let stream = transport.into_inner();
                      let mut transport = webcontr::transport::tcp::response_transport(stream);

                      let response_frame = webcontr::prelude::StreamExt::next(&mut transport).await.unwrap().unwrap();

                      let thing: #rpc_res_ident = match response_frame {
                          webcontr::transport::tcp::frame::ResponseFrame::Error(err) => return Err(err),
                          webcontr::transport::tcp::frame::ResponseFrame::Payload(data) => bincode::deserialize(&data).unwrap(),
                      };



                      match thing {
                          #rpc_res_ident::#rpc_ident(response) => Ok(response),
                          _ => unreachable!()
                      }
                  }
              )*
    //async fn hello(&mut self, value: String, value1: String) -> String {
    //  let stream = TcpStream::connect(&self.addr).await.unwrap();
    //  let mut transport = tcp::request_transport(stream);
    //
    //  let req = PingCommandRequest::hello { value, value1 };
    //
    //  let body = bincode::serialize(&req).unwrap();
    //
    //  transport
    //    .send(RequestFrame::new("PingCommand".to_string(), Bytes::from(body)))
    //    .await
    //    .unwrap();
    //
    //  let stream = transport.into_inner();
    //
    //  let mut transport = tcp::response_transport(stream);
    //
    //  let response_frame = transport.next().await.unwrap().unwrap();
    //
    //  let thing: PingCommandResponse = match response_frame {
    //    ResponseFrame::Error(err) => match err {
    //      ResponseErrorKind::MethodNotFound => {
    //        panic!("webcontr error: Command not found :(")
    //      }
    //      ResponseErrorKind::InvalidRequest => panic!("webcontr error: whaat"),
    //    },
    //  };
    //  match thing {
    //    PingCommandResponse::hello(res) => res,
    //    _ => unreachable!(),
    //  }
    //}
          }
      }
  }
}

impl ToTokens for ServiceGenerator {
  fn to_tokens(&self, tokens: &mut TokenStream2) {
    tokens.extend([
      self.service_response.to_token_stream(),
      self.service_request.to_token_stream(),
      self.trait_service(),
      self.impl_serve_fn(),
      self.impl_servicename_fn(),
      self.service_client(),
    ]);
  }
}
