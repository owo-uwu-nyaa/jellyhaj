use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_text(container: &Container, exports: &Path) -> TokenStream {
    let accepts_inputs = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {#i => #exports::JellyhajWidget::<R>::accepts_text_input(&self.#var),}
    });
    let accept_char = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {#i => #exports::JellyhajWidget::<R>::accept_char(&mut self.#var, text),}
    });
    let accept_text = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {#i => #exports::JellyhajWidget::<R>::accept_text(&mut self.#var, text),}
    });
    quote! {
        fn accepts_text_input(&self, current: #exports::usize) -> #exports::bool{
            match current {
                #(#accepts_inputs)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            }
        }
        fn accept_char(&mut self, text: #exports::char, current: #exports::usize){
            match current {
                #(#accept_char)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            }
        }
        fn accept_text(&mut self, text: #exports::String, current: #exports::usize){
            match current {
                #(#accept_text)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            }
        }
    }
}
