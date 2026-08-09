use proc_macro2::TokenStream;
use quote::quote;

use crate::form::{Component, FieldKind};

impl Component {
    #[must_use]
    pub fn make_action(&self) -> TokenStream {
        let vis = &self.original.vis;
        let action = &self.action;
        let form_item_base = &self.paths.form_item_base;
        let component = &self.paths.form_component;
        let items = self.fields.iter().map(|item| {
            let name = &item.enum_id;
            let ty = &item.ty;
            match &item.kind {
                FieldKind::Item { descr: _ } => quote! {#name(<#ty as #form_item_base>::Action)},
                FieldKind::Flatten => quote! {#name(<#ty as #component>::Action)},
            }
        });
        let exports = &self.paths.exports;
        quote! {
            #[derive(#exports::Debug)]
            #vis enum #action {
                #(#items),*
            }
        }
    }
}
