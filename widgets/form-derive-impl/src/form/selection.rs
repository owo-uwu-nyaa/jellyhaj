use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::LitStr;

use crate::form::{Component, FieldKind};

impl Component {
    #[must_use]
    pub fn make_selection_ty(&self) -> TokenStream {
        let vis = &self.original.vis;
        let exports = &self.paths.exports;
        let selection = &self.selection;
        let item_base = &self.paths.form_item_base;
        let component = &self.paths.form_component;
        let items = self.fields.iter().map(|item| {
            let name = &item.enum_id;
            let ty = &item.ty;
            match &item.kind {
                FieldKind::Item { descr: _ } => quote! {
                    #name(<#ty as #item_base>::SelectionInner)
                },
                FieldKind::Flatten => quote! {
                    #name(<#ty as #component>::Selector)
                },
            }
        });
        quote! {
            #[derive(#exports::Debug)]
            #vis enum #selection {
                #(#items),*
            }
        }
    }

    #[must_use]
    pub fn make_selection_default(&self) -> TokenStream {
        let exports = &self.paths.exports;
        let selection = &self.selection;
        let pat = &self
            .fields
            .first()
            .expect("component with 0 fields")
            .enum_id;
        quote! {
            #[automatically_derived]
            impl #exports::Default for #selection {
                fn default() -> Self{
                    Self::#pat(#exports::Default::default())
                }
            }
        }
    }

    #[must_use]
    pub fn make_selection_valuable(&self) -> TokenStream {
        let exports = &self.paths.exports;
        let selection = &self.selection;
        let name = LitStr::new(&selection.to_string(), selection.span());
        let defs = self.fields.iter().map(|item| {
            let name = LitStr::new(&item.enum_id.to_string(), item.enum_id.span());
            quote! {#exports::VariantDef::new(#name, #exports::Fields::Unnamed(1))}
        });

        let visit_pats = self.fields.iter().map(|item| {
            let sel = &item.selection;
            quote! {#sel(v) => #exports::Valuable::as_value(v)}
        });
        let var_pats = self.fields.iter().enumerate().map(|(i, item)| {
            let i = Literal::usize_suffixed(i);
            let sel = &item.selection;
            quote! {
                #sel(_) => {
                    #exports::Variant::Static(&DEFS[#i])
                }
            }
        });
        quote! {
            const _: () = {
                static DEFS: &[#exports::VariantDef] = &[
                    #(#defs),*
                ];
                #[automatically_derived]
                impl #exports::Valuable for #selection{
                    fn as_value(&self) -> #exports::Value<'_>{
                        #exports::Value::Enumerable(self)
                    }
                    fn visit(&self, visit: &mut dyn #exports::Visit){
                        let val = match self{
                            #(#visit_pats),*
                        };
                        visit.visit_unnamed_fields(&[val])
                    }
                }
                #[automatically_derived]
                impl #exports::Enumerable for #selection {
                fn definition(&self) -> #exports::EnumDef<'_>{
                    #exports::EnumDef::new_static(
                        #name, DEFS
                    )
                }
                fn variant(&self) -> #exports::Variant<'_>{
                    match self{
                        #(#var_pats)*
                    }
                }
            }
            };
        }
    }
}
