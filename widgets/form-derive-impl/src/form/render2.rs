use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path, Type};

use crate::form::FormItem;

pub fn pass2_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    size_helpers: &Path,
    exports: &Path,
    form_item_tr: &Type,
) -> TokenStream {
    let render = items.iter().map(|item| {
        let ty = &item.ty;
        let pat = &item.selection;
        let id = &item.name;
        let descr = &item.descr;
        let render = quote! {
        };
        if let Some(show_if_fun) = item.show_if_fun.as_ref() {
            quote! {
                if state.#show_if_fun(){
                    #render
                }
            }
        } else {
            render
        }
    });

    quote! {
        pub fn #name(
            state: &#state_ty,
            sel: &#selection_ty,
            buf: &mut #exports::Buffer,
            mut area: #exports::Rect,
            max_height: u16,
        ){
            let mut first = true;
            #(#render)*
        }
    }
}
