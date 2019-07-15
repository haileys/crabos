extern crate proc_macro;
extern crate proc_macro2;
extern crate syn;

use proc_macro::TokenStream;
use quote::quote;
use syn::DeriveInput;

#[proc_macro_derive(PageSized)]
pub fn page_sized(input: TokenStream) -> TokenStream {
    let ast = syn::parse::<DeriveInput>(input).expect("syn::parse");

    let ty_name = &ast.ident;

    let tokens = quote! {
        const _: [(); 0 - {
            (::core::mem::size_of::<#ty_name>() > crate::mem::page::PAGE_SIZE) as usize
        }] = [];

        unsafe impl crate::mem::kvirt::PageSized for #ty_name {}
    };

    tokens.into()
}
