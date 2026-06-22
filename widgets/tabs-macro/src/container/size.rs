use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_size(container: &Container, exports: &Path) -> TokenStream {
    let min_widths = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {
            #i => #exports::JellyhajWidget::<R>::min_width(&self.#var),
        }
    });
    let min_heights = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {
            #i => #exports::JellyhajWidget::<R>::min_height(&self.#var),
        }
    });

    quote! {
        fn min_width(&self, current: usize) -> #exports::Option<#exports::u16>{
            match current {
                #(#min_widths)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            }
        }
        fn min_height(&self, current: usize) -> #exports::Option<#exports::u16>{
            match  current {
                #(#min_heights)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            }
        }
    }
}
