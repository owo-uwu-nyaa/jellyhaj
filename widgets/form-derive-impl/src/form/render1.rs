use proc_macro2::TokenStream;
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
    height_store_ty: &Ident,
) -> TokenStream {
    let render = items.iter().map(|item| {
        let ty = &item.ty;
        let pat = &item.selection;
        let id = &item.name;
        let descr = &item.descr;
        quote! {
            {
                if let Some(y) = store.#id{
                    let height = <#ty as #form_item_tr>::HEIGHT;
                    let mut this_area = area;
                    this_area.height = height;
                    this_area.y = y; <#ty as #form_item_tr>::render_pass_main(
                        &state.#id,
                        this_area,
                        buf,
                        #exports::matches!(sel, #pat(_)),
                        #descr
                    );
                }
            }
        }
    });

    quote! {
        pub fn #name(
            state: &#state_ty,
            sel: &#selection_ty,
            buf: &mut #exports::Buffer,
            area: #exports::Rect,
            store: &mut #height_store_ty,
        ){
            #(#render)*
        }
    }
}
