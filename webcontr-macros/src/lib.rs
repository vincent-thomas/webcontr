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
               println!("server request: {:?}", &req);
               let req: #req_ident = webcontr::prelude::bincode::deserialize(&req).map_err(|_| webcontr::ServeError::InvalidRequest)?;
               match req {
                   #(
                    #req_ident::#variants { #(#rpcs_args),* } => {
                        let out = #ident::#variants(&self.service, #(#rpcs_args),*).await;
                        let bytes_vec = webcontr::prelude::bincode::serialize(&#res_ident::#variants(out)).unwrap();
                        let bytes = webcontr::prelude::Bytes::from(bytes_vec);
                        println!("server response: {:?}", &bytes);
                        Ok(bytes)
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
                pub async fn #rpc_ident(&mut self, #(#rpc_args_types),*) -> Result<#rpc_return_type, webcontr::ClientError> {
                    let stream = webcontr::prelude::TcpStream::connect(&self.addr).await.map_err(|err| webcontr::ClientError::IoError(err))?;
                    let (read, mut write) = stream.into_split();
                    let mut transport = webcontr::transport::tcp::client::request_transport(write);

                    let req = #rpc_req_ident::#rpc_ident { #(#rpc_args),* };
                    let body = webcontr::prelude::bincode::serialize(&req).map_err(|err| webcontr::ClientError::EncodingError(err))?;

                    let request_frame = webcontr::transport::frame::RequestFrame::new(
                        stringify!(#ident).to_string(),
                        webcontr::prelude::Bytes::from(body)
                    );

                    webcontr::prelude::SinkExt::send(&mut transport, request_frame).await.unwrap();

                    let stream = transport.into_inner();
                    let mut transport = webcontr::transport::tcp::client::response_transport(read);

                    let response_frame = webcontr::prelude::StreamExt::next(&mut transport).await.unwrap().unwrap();

                    let thing: #rpc_res_ident = match response_frame {
                        webcontr::transport::frame::ResponseFrame::Error(err) => return Err(webcontr::ClientError::ServerError(err)),
                        webcontr::transport::frame::ResponseFrame::Payload(data) => webcontr::prelude::bincode::deserialize(&data).unwrap(),
                    };



                    match thing {
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
