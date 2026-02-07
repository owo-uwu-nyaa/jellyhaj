use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, Path, Type};

use super::FormItem;

pub fn height_fn(
    items: &[FormItem],
    state_ty: &Ident,
    name: &Ident,
    size_helpers: &Path,
    form_item_tr: &Type,
    action_result_ty: &Type,
    height_store_ty: &Type,
) -> TokenStream {
    let calc = items.iter().enumerate().map(|(i, item)| {
        let ty = &item.ty;
        let index = Literal::usize_suffixed(i);
        let calc = quote! {
            if first{
                first = false;
                height = <#ty as #form_item_tr>::HEIGHT;
                height_buf = <#ty as #form_item_tr>::HEIGHT_BUF;
            }else{
                height = #size_helpers::add_form_item::<#action_result_ty,#ty>(height);
                height_buf = #size_helpers::add_form_item_buf::<#action_result_ty,#ty>(height_buf);
            }
            store[#index] = (height, true);
        };
        if let Some(show_if_fun) = item.show_if_fun.as_ref() {
            quote! {
                if state.#show_if_fun(){
                    #calc
                }else{
                    store[#index] = (height, false);
                }
            }
        } else {
            calc
        }
    });

    quote! {
        fn #name(state: &#state_ty, store: &mut #height_store_ty)->u16{
            let mut first = true;
            let mut height = 0;
            let mut height_buf = 0;

            #(#calc)*

            #size_helpers::add(height, height_buf)
        }
    }
}

pub fn item_start_fn(
    items: &[FormItem],
    selection_ty: &Ident,
    height_store_ty: &Type,
    name: &Ident,
) -> TokenStream {
    let pats = items.iter().enumerate().map(|(i, item)| {
        let pat = &item.selection;
        let index = Literal::usize_suffixed(i);
        quote! {
            #pat(_) => #index
        }
    });
    quote! {
        fn #name(store: &#height_store_ty, sel: #selection_ty)->u16{
            let index = match sel{
                #(#pats),*
            };
            store[index].0
        }
    }
}
