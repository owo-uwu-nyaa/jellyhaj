use proc_macro2::TokenStream;
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

        }
    }
}
