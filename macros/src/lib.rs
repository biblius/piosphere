use quote::quote;
use syn::{ItemStruct, Token};

#[proc_macro_attribute]
#[proc_macro_error::proc_macro_error]
pub fn request(
    attrs: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let strct: ItemStruct = syn::parse(input).expect("invalid input");

    let id = &strct.ident;
    let Request { response, tag } = syn::parse(attrs).expect("invalid input");

    quote!(
        #strct

        impl crate::socket::Message for #id {
            type Response = #response;

            fn tag(&self) -> crate::socket::PiosphereTag {
                crate::socket::PiosphereTag::#tag
            }
        }
    )
    .into()
}

#[derive(Debug)]
struct Request {
    response: syn::Path,
    tag: syn::Path,
}

impl syn::parse::Parse for Request {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let response: syn::Path = input.parse()?;
        input.parse::<Token![,]>()?;
        let tag: syn::Path = input.parse()?;
        Ok(Self { response, tag })
    }
}
