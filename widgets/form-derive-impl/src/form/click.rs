use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, Path, Type};

use crate::form::FormItem;

pub fn click_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    height_store_ty: &Type,
    name: &Ident,
    exports: &Path,
    form_item_tr: &Type,
    action_result_ty: &Type,
    assert_current_shown_fn: &Ident,
    size_helpers: &Path,
) -> TokenStream {
    let current = items.iter().enumerate().map(|(i, item)| {
        let pat = &item.selection;
        let name = &item.name;
        let index = Literal::usize_suffixed(i);
        let ty = &item.ty;
        quote! {
            #pat(inner_sel) => {
                let this_area = #exports::Rect{
                    x: 0,
                    y: store[#index].0 - offset,
                    width: size.width,
                    height: <#ty as #form_item_tr>::HEIGHT
                };
                let active = <#ty as #form_item_tr>::popup_area(
                    &state.#name,
                    *inner_sel,
                    this_area,
                    size
                );
                if (active.height - active.y > position.y) &&
                   (active.width  - active.x > position.x){
                    let res = <#ty as #form_item_tr>::apply_click_active(
                        &mut state.#name,
                        inner_sel,
                        this_area,
                        size,
                        position,
                        kind,
                        modifier
                    )?;
                    #assert_current_shown_fn(state, *sel);
                    return #exports::Result::Ok(res);
                }
            }
        }
    });

    let next = items.iter().enumerate().map(|(i, item)| {
        let index = Literal::usize_suffixed(i);
        let ty = &item.ty;
        let name = &item.name;
        let pat = &item.selection;
        quote! {
            #index => {
                let base = position.y - store[#index].0;
                if base < <#ty as #form_item_tr>::HEIGHT{
                    let (s, res) =<#ty as #form_item_tr>::apply_click_inactive(
                        &mut state.#name,
                        #exports::Size{
                            width: size.width,
                            height: <#ty as #form_item_tr>::HEIGHT,
                        },
                        #exports::Position{
                            x: position.x,
                            y: base
                        },
                        kind,
                        modifier
                    )?;
                    if let Some(s)=s{
                        *sel = #pat(s);
                    }
                    #assert_current_shown_fn(state, *sel);
                    return #exports::Result::Ok(res);
                }
            }
        }
    });

    quote! {
        #[allow(clippy::too_many_arguments)]
        fn #name(
            state: &mut #state_ty,
            sel: &mut #selection_ty,
            store: &#height_store_ty,
            position: #exports::Position,
            size: #exports::Size,
            kind: #exports::MouseEventKind,
            modifier: #exports::KeyModifiers,
            offset: u16,
        )-> #exports::Result<#exports::Option<#action_result_ty>>{
            match sel{
                #(#current),*
            }
            match #size_helpers::find_height(store, position.y){
                #(#next),*
                _ => unreachable!()
            }

            todo!()
        }
    }
}
