use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_tabbed(container: &Container, exports: &Path) -> TokenStream {
    let ty = &container.ty;
    let en_ty = &container.en_ty;
    let result_ty = &container.result_ty;
    let tab_names = container.tabs.iter().map(|v| &v.name);

    let visitor = container.tabs.iter().map(|v| {
        let var = &v.var;
        let ty = &v.ty;
        quote! {#exports::WidgetTreeVisitor::visit::<#ty>(visitor, &self.#var);}
    });

    let min_widths = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {
            #i => #exports::JellyhajWidgetBase::min_width(&self.#var),
        }
    });
    let min_heights = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {
            #i => #exports::JellyhajWidgetBase::min_height(&self.#var),
        }
    });

    let accepts_inputs = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {#i => #exports::JellyhajWidgetBase::accepts_text_input(&self.#var),}
    });
    let accept_char = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {#i => #exports::JellyhajWidgetBase::accept_char(&mut self.#var, text),}
    });
    let accept_text = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        quote! {#i => #exports::JellyhajWidgetBase::accept_text(&mut self.#var, text),}
    });

    quote! {
        #[automatically_derived]
        impl #exports::Tabbed for #ty {
            type Action = #en_ty;
            type ActionResult = #result_ty;

            const TABS: &[&str] = &[#(#tab_names),*];

            fn is_next(action: &Self::Action) -> bool {#exports::matches!(action, #en_ty::Next)}
            fn is_prev(action: &Self::Action) -> bool {#exports::matches!(action, #en_ty::Prev)}

            fn visit_children(&self, visitor: &mut impl #exports::WidgetTreeVisitor){
                #(#visitor)*
            }

            fn min_width(&self, current: usize) -> #exports::Option<#exports::u16>{
                match current {
                    #(#min_widths)*
                    _ => #exports::unreachable!("should only be called with current in bounds")
                }
            }
            fn min_height(&self, current: usize) -> #exports::Option<#exports::u16>{
                match  current {
                    #(#min_heights)*
                    _ => #exports::unreachable!("should only be called with current in bounds")
                }
            }

            fn accepts_text_input(&self, current: #exports::usize) -> #exports::bool{
                match current {
                    #(#accepts_inputs)*
                    _ => #exports::unreachable!("should only be called with current in bounds")
                }
            }
            fn accept_char(&mut self, text: #exports::char, current: #exports::usize){
                match current {
                    #(#accept_char)*
                    _ => #exports::unreachable!("should only be called with current in bounds")
                }
            }
            fn accept_text(&mut self, text: #exports::String, current: #exports::usize){
                match current {
                    #(#accept_text)*
                    _ => #exports::unreachable!("should only be called with current in bounds")
                }
            }
        }
    }
}
