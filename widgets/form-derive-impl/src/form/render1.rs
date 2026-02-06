use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, Path, Type};

use crate::form::FormItem;

pub fn pass1_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    name: &Ident,
    exports: &Path,
    form_item_tr: &Type,
    height_store_ty: &Type,
) -> TokenStream {
    let render = items.iter().enumerate().map(|(i, item)| {
        let ty = &item.ty;
        let pat = &item.selection;
        let id = &item.name;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            {
                if store[#index].1{
                    let mut this_area = area;
                    this_area.height = <#ty as #form_item_tr>::HEIGHT;
                    this_area.y += store[#index].0;
                    <#ty as #form_item_tr>::render_pass_main(
                        &mut state.#id,
                        this_area,
                        buf,
                        #exports::matches!(sel, #pat(_)),
                        #descr
                    )?;
                }
            }
        }
    });

    quote! {
        pub fn #name(
            state: &mut #state_ty,
            sel: #selection_ty,
            buf: &mut #exports::Buffer,
            area: #exports::Rect,
            store: &#height_store_ty,
        )-> #exports::Result<()>{
            #(#render)*
            #exports::Result::Ok(())
        }
    }
}
