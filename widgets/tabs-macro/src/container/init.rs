use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_init(container: &Container, exports: &Path) -> TokenStream {
    let inits = container.tabs.iter().map(|v| {
        let var = &v.var;
        let pat = &v.en_pat;
        quote! {#exports::JellyhajWidget::init(&mut self.#var, cx.wrap_with(#pat));}
    });
    quote! {
        fn init(
            &mut self,
            cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>
        ) {
            #(#inits)*
        }
    }
}
