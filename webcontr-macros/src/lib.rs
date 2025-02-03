mod service;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use service::{
  res_req::{ServiceRequest, ServiceResponse},
  Service,
};
use syn::{parse_macro_input, Ident, Pat, PatType, ReturnType};

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
      &format!("{}Serve", self.service.ident),
      self.service.ident.span(),
    );
    quote! {
       #(#attrs_iter),*
       #[webcontr::async_trait]
       #vis trait #ident: Sized + Clone {
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
      &format!("{}Serve", self.service.ident),
      self.service.ident.span(),
    );
    quote! {
        #[derive(Clone)]
        #vis struct #serve_struct_ident<S: Clone> {
            pub service: S
        }

        impl<A: #ident + Send + Clone + Sync + 'static> Service<Bytes>
          for #serve_struct_ident<A>
        {
          type Response = Bytes;
          type Error = webcontr::transport::frame::ResponseErrorKind;
          type Future =
            std::pin::Pin<Box<dyn std::future::Future<Output = Result<Self::Response, Self::Error>> + Send>>;

          fn poll_ready(
            &mut self,
            _cx: &mut std::task::Context<'_>,
          ) -> std::task::Poll<Result<(), Self::Error>> {
            // Service is always ready
            std::task::Poll::Ready(Ok(()))
          }

          fn call(&mut self, req: webcontr::prelude::Bytes) -> Self::Future {
            let service = self.service.clone();

            Box::pin(async move {
              let req: #req_ident = bincode::deserialize(&req)
                .map_err(|_| ResponseErrorKind::InvalidRequest)?;

              match req {
                #(
                  #req_ident::#variants { #(#rpcs_args),* } => {
                    let out = #ident::#variants(&service, #(#rpcs_args),*).await;
                    let bytes_vec =
                      bincode::serialize(&#res_ident::#variants(out)).unwrap();
                    Ok(Bytes::from(bytes_vec))
                  }
                ),*
              }
            })
          }
        }
    }
  }

  fn impl_servicename_fn(&self) -> TokenStream2 {
    let ident = &self.service.ident;
    let serve_struct_ident = Ident::new(
      &format!("{}Serve", self.service.ident),
      self.service.ident.span(),
    );

    quote! {
            impl<A: Clone> webcontr::ServiceName for #serve_struct_ident<A> {
            fn name(&self) -> &'static str {
                stringify!(#ident)
            }
        }
    }
  }

  fn service_client(&self) -> TokenStream2 {
    let ident = &self.service.ident;

    let client_ident = Ident::new(
      &format!("{}Client", self.service.ident),
      self.service.ident.span(),
    );

    let rpc_attrs = self.service.rpcs.iter().map(|rpc| rpc.attrs.clone());
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
                #(#rpc_attrs)*
                pub async fn #rpc_ident(&mut self, #(#rpc_args_types),*) -> Result<#rpc_return_type, webcontr::ClientError> {

                    let req = #rpc_req_ident::#rpc_ident { #(#rpc_args),* };
                    let res: #rpc_res_ident = webcontr::transport::tcp::client::send_client_req(stringify!(#ident), req, &self.addr).await?;

                    match res {
                        #rpc_res_ident::#rpc_ident(response) => Ok(response),
                        _ => unreachable!()
                    }
                }
            )*
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
