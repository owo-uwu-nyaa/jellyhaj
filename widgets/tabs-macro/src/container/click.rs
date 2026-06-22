use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_click(container: &Container, exports: &Path) -> TokenStream {
    let result_ty = &container.result_ty;
    let click = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        let pat = &v.en_pat;
        quote! {#i => #exports::JellyhajWidget::click(
            &mut self.#var,
            cx.wrap_with(#pat),
            position,
            size,
            kind,
            modifier,
        ).map(|v|v.map(|v|Into::<#result_ty>::into(v))),
        }
    });

    quote! {
        fn click(
            &mut self,
            cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
            position: #exports::Position,
            size: #exports::Size,
            kind: #exports::MouseEventKind,
            modifier: #exports::KeyModifiers,
            current: #exports::usize,
        ) -> #exports::Result<#exports::Option<Self::ActionResult>>{
            match current {
                #(#click)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            }
        }
    }
}
