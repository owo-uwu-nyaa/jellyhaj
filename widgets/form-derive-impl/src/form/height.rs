use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path, Type};

use super::FormItem;

pub fn make_height_store(items: &[FormItem], name: &Ident, exports: &Path) -> TokenStream {
    let items = items.iter().map(|i| &i.name);
    let items2 = items.clone();
    quote! {
        struct #name {
            #(#items: #exports::Option<u16>),*
        }

        impl #exports::Default for #name{
            fn default() -> Self{
                Self{
                    #(#items2: #exports::None),*
                }
            }
        }
    }
}

pub fn height_fn(
    items: &[FormItem],
    state_ty: &Ident,
    name: &Ident,
    size_helpers: &Path,
    exports: &Path,
    form_item_tr: &Type,
    action_result_ty: &Type,
    height_store_ty: &Ident,
) -> TokenStream {
    let calc = items.iter().map(|item| {
        let ty = &item.ty;
        let name = &item.name;
        let calc = quote! {
            if first{
                first = false;
                height = <#ty as #form_item_tr>::HEIGHT;
                height_buf = <#ty as #form_item_tr>::HEIGHT_BUF;
            }else{
                height = #size_helpers::add_form_item::<#action_result_ty,#ty>(height);
                height_buf = #size_helpers::add_form_item_buf::<#action_result_ty,#ty>(height_buf);
            }
            store.#name = #exports::Some(height);
        };
        if let Some(show_if_fun) = item.show_if_fun.as_ref() {
            quote! {
                if state.#show_if_fun(){
                    #calc
                }else{
                    store.#name = #exports::None;
                }
            }
        } else {
            calc
        }
    });

    quote! {
        pub fn #name(state: &#state_ty, store: &mut #height_store_ty)->u16{
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
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    size_helpers: &Path,
    form_item_tr: &Type,
    action_result_ty: &Type,
) -> TokenStream {
    let calc = items.iter().map(|item| {
        let ty = &item.ty;
        let calc = quote! {
            if first{
                first = false;
                height = <#ty as #form_item_tr>::HEIGHT;
            }else{
                height = #size_helpers::add_form_item::<#action_result_ty,#ty>(height);
            }
        };
        let calc = if let Some(show_if_fun) = item.show_if_fun.as_ref() {
            quote! {
                if state.#show_if_fun(){
                    #calc
                }
            }
        } else {
            calc
        };
        let pat = &item.selection;
        quote! {
            if let #pat(_) = sel{
                return height;
            }
            #calc
        }
    });
    quote! {
        pub fn #name(state: &#state_ty, sel: &#selection_ty)->u16{
            let mut first = true;
            let mut height = 0;

            #(#calc)*

            unreachable!()
        }
    }
}
