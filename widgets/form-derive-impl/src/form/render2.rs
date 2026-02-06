use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Ident, Path, Type};

use crate::form::FormItem;

pub fn pass2_fn(
    items: &[FormItem],
    state_ty: &Ident,
    selection_ty: &Ident,
    height_store_ty: &Type,
    name: &Ident,
    exports: &Path,
    form_item_tr: &Type,
) -> TokenStream {
    let render = items.iter().enumerate().map(|(i,item)| {
        let ty = &item.ty;
        let pat = &item.selection;
        let name = &item.name;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            #pat(sel) => {
                let full_area = area;
                let mut this_area = area;
                this_area.height = <#ty as #form_item_tr>::HEIGHT;
                this_area.y += (store[#index].0-offset);
                <#ty as #form_item_tr>::render_pass_popup(
                     &mut state.#name,
                     this_area,
                     full_area,   
                    buf,
                     #descr,
                     sel
                )
            }
        }
    });

    quote! {
        pub fn #name(
            state: &mut #state_ty,
            sel: #selection_ty,
            store: &#height_store_ty,
            buf: &mut #exports::Buffer,
            area: #exports::Rect,
            offset: u16,
        )->#exports::Result<()>{
            match sel{
                #(#render),*
            }
        }
    }
}
