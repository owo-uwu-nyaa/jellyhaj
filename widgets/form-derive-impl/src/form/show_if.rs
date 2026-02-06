use proc_macro2::{Literal, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Ident, Path, spanned::Spanned};

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

pub fn assert_current_shown_fn(
    items: &[FormItem],
    selection_ty: &Ident,
    state_ty: &Ident,
    name: &Ident,
    exports: &Path,
) -> TokenStream{
    let asserts = items.iter().map(|item|{
        let pat = &item.selection;
        if let Some(if_fn) = item.show_if_fun.as_ref(){
            let message = Literal::string(&format!("action on {} resulted in it beeing hidden", item.name));
            quote! {
                #pat(_) => #exports::assert!(state.#if_fn(), #message)
            }
        }else{
            quote! {
                #pat(_) => {}
            }
        }
    });
    quote! {
        fn #name(state:&#state_ty, sel: #selection_ty){
            match sel{
                #(#asserts),*
            }
        }
    }
}
