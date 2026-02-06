use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Path, Type};

use crate::form::FormItem;

pub fn make_selection(
    items: &[FormItem],
    selection_ty: &Ident,
    form_item_tr: &Type,
    exports: &Path,
) -> TokenStream {
    let first_pat = &items.first().expect("state struct is empty").selection_id;
    let items = items.iter().map(|item| {
        let name = &item.selection_id;
        let ty = &item.ty;
        quote! {#name(<#ty as #form_item_tr>::SelectionInner)}
    });

    quote! {
        #[derive(Clone,Copy)]
        pub enum #selection_ty{
            #(#items),*
        }

        impl #exports::Default for #selection_ty{
            fn default() -> Self{
                Self::#first_pat(#exports::Default::default())
            }
        }
    }
}

pub fn detect_loop_fn(
    items: &[FormItem],
    selection_ty: &Ident,
    name: &Ident,
    exports: &Path,
) -> TokenStream {
    let same = items.iter().map(|i| {
        let pat = &i.selection;
        quote! {
            (#pat(_), #pat(_))
        }
    });
    quote! {
        fn #name(s1: #selection_ty, s2: #selection_ty){
            match (s1, s2) {
                #(#same)|* => #exports::panic!("no available state to move to"),
                _ => {}
            }
        }
    }
}

pub fn up_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    exports: &Path,
    detect_loop_fn: &Ident,
) -> TokenStream {
    let map = (0..items.len()).map(|i| {
        let cur = &items[i];
        let cur_pat = &cur.selection;
        let next_pat = &items[(i + 1) % items.len()].selection;
        if let Some(cur_if) = &cur.show_if_fun {
            quote! {
                #next_pat(_) =>(#cur_pat(#exports::Default::default()), state.#cur_if())
            }
        } else {
            quote! {
                #next_pat(_) =>(#cur_pat(#exports::Default::default()), true)
            }
        }
    });

    quote! {
        fn #name(state: &#state_ty, mut sel: #selection_ty) -> #selection_ty{
            let initial = sel;
            loop{
                let visible;
                (sel, visible) = match sel {
                    #(#map),*
                };
                if visible {
                    break sel;
                }
                #detect_loop_fn(initial, sel);
            }
        }

    }
}

pub fn down_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    exports: &Path,
    detect_loop_fn: &Ident,
) -> TokenStream {
    let map = (0..items.len()).map(|i| {
        let cur_pat = &items[i].selection;
        let next = &items[(i + 1) % items.len()];
        let next_pat = &next.selection;
        if let Some(next_if) = &next.show_if_fun {
            quote! {
                #cur_pat(_) =>(#next_pat(#exports::Default::default()), state.#next_if())
            }
        } else {
            quote! {
                #cur_pat(_) =>(#next_pat(#exports::Default::default()), true)
            }
        }
    });

    quote! {
        fn #name(state: &#state_ty, mut sel: #selection_ty) -> #selection_ty{
            let initial = sel;
            loop{
                let visible;
                (sel, visible) = match sel {
                    #(#map),*
                };
                if visible {
                    break sel;
                }
                #detect_loop_fn(initial, sel);
            }
        }

    }
}

pub fn with_current_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    with_current_helpers: &Path,
    action_result: &Type,
) -> TokenStream {
    let pats = items.iter().map(|item| {
        let pat = &item.selection;
        let name = &item.name;
        let ty = &item.ty;
        quote! {
            #pat(sel) => #with_current_helpers::WithCurrent::process::<#ty>(
                f,
                &state.#name,
                sel
            )
        }
    });
    quote! {
        fn #name<W: #with_current_helpers::WithCurrent<#action_result>>(
            state: &#state_ty,
            sel: #selection_ty,
            f: W
        )-> W::R{
            match sel {
                #(#pats),*
            }
        }
    }
}

pub fn with_current_mut_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    with_current_helpers: &Path,
    action_result: &Type,
) -> TokenStream {
    let pats = items.iter().map(|item| {
        let pat = &item.selection;
        let name = &item.name;
        let ty = &item.ty;
        quote! {
            #pat(sel) => #with_current_helpers::WithCurrentMut::process::<#ty>(
                f,
                &mut state.#name,
                sel
            )
        }
    });
    quote! {
        fn #name<W: #with_current_helpers::WithCurrentMut<#action_result>>(
            state: &mut #state_ty,
            sel: &mut #selection_ty,
            f: W
        )-> W::R{
            match sel {
                #(#pats),*
            }
        }
    }
}
