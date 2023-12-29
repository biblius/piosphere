use proc_macro_error::abort;
use quote::quote;
use syn::{spanned::Spanned, Data, DataEnum, DeriveInput, Ident, MetaList};

#[proc_macro_derive(PiteriaRequest, attributes(response))]
#[proc_macro_error::proc_macro_error]
pub fn derive_request(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let derive: DeriveInput = syn::parse(input).expect("invalid input");

    let Data::Enum(DataEnum { variants, .. }) = derive.data else {
        abort!(derive.span(), "PiteriaRequest must be an enum");
    };

    let mut res_variants: Vec<(&Ident, Option<&MetaList>)> = vec![];

    for variant in variants.iter() {
        let ident = &variant.ident;

        if variant.attrs.is_empty() {
            res_variants.push((ident, None));
            continue;
        }

        let attr = &variant.attrs[0];

        if !attr.meta.path().is_ident("response") {
            abort!(attr.span(), "Unrecognized annotation")
        }

        let list = attr.meta.require_list().expect("must be list");

        res_variants.push((ident, Some(list)));
    }

    let tokens = res_variants.into_iter().map(|(id, ty)| match ty {
        Some(list) => {
            let ty = list.parse_args::<syn::Type>().expect("must be type");
            quote!(#id(#ty))
        }
        None => quote!(#id),
    });

    quote!(
        #[derive(Debug, Serialize, Deserialize)]
        pub enum PiteriaResponse {
            #(#tokens),*
        }
    )
    .into()
}
