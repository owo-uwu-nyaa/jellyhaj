use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_render(container: &Container, exports: &Path) -> TokenStream {
    let render = container.tabs.iter().enumerate().map(|(i, v)| {
        let var = &v.var;
        let i = Literal::usize_suffixed(i);
        let pat = &v.en_pat;
        quote! {#i => #exports::JellyhajWidgetExt::render_fallible(
            &mut self.#var,
            area,
            buf,
            cx.wrap_with(#pat)
        ),}
    });
    quote! {
        fn render_fallible(
            &mut self,
            area: #exports::Rect,
            buf: &mut #exports::Buffer,
            cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
            current: #exports::usize,
        ) -> #exports::Result<()>{
            match current {
                #(#render)*
                _ => #exports::unreachable!("should only be called with current in bounds")

            }
        }
    }
}
