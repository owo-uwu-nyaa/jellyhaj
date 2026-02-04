use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::{Ident, spanned::Spanned};

use crate::form::FormItem;

pub fn gen_show_ifs(items: &[FormItem], self_ty: &Ident) -> TokenStream {
    let fns = items.iter().filter_map(|item| {
        if let (Some(name), Some(expr)) = (item.show_if_fun.as_ref(), item.show_if.as_ref()) {
            let span = expr.span();
            Some(quote_spanned! {span=> pub fn #name(&self)->bool{
                #expr
            }})
        } else {
            None
        }
    });

    quote! {
        impl #self_ty {
            #(#fns)*
        }
    }
}
