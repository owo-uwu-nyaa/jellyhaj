use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_enum(container: &Container, exports: &Path) -> TokenStream {
    let en_ty = &container.en_ty;
    let vis = &container.vis;
    let universal = container.common_action.iter();
    let en_decl = container.tabs.iter().map(|v| {
        let id = &v.en_id;
        let ty = &v.ty;
        quote! {#id(<#ty as #exports::JellyhajWidgetBase>::Action)}
    });

    quote! {
        #[derive(#exports::Debug)]
        #vis enum #en_ty {
            Next,
            Prev,
            #(Universal(#universal),)*
            #(#en_decl),*
        }
    }
}
