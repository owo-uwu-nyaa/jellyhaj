use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_action(container: &Container, exports: &Path) -> TokenStream {
    let en_ty = &container.en_ty;
    let result_ty = &container.result_ty;
    let action_pat = container.tabs.iter().map(|v| {
        let var = &v.var;
        let pat = &v.en_pat;
        quote! {
            #pat(v) => #exports::JellyhajWidget::apply_action(
                &mut self.#var,
                cx.wrap_with(#pat),
                v
            ).map(|v|v.map(|v|#exports::Into::<#result_ty>::into(v))) ,
        }
    });
    let universal = container.common_action.iter().map(|_|{
        let pats = container.tabs.iter().enumerate().map(|(i,v)|{
            let i = Literal::usize_suffixed(i);
            let var = &v.var;
            let pat = &v.en_pat;
            let ty = &v.ty;
            quote! {
                #i => if let #exports::Some(action) =
                #exports::Into::<#exports::Option<<#ty as #exports::JellyhajWidgetBase>::Action>>::into(action) {
                    #exports::JellyhajWidget::apply_action(
                        &mut self.#var,
                        cx.wrap_with(#pat),
                        action
                    ).map(|v|v.map(|v|#exports::Into::<#result_ty>::into(v)))
                } else {
                    #exports::Ok(#exports::None)
                },
            }
        });
        quote! {
            #en_ty::Universal(action) => match current{
                #(#pats)*
                _ => #exports::unreachable!("should only be called with current in bounds")
            },
        }
    });
    quote! {
        fn apply_action(
            &mut self,
            cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
            action: Self::Action,
            current: usize,
        ) -> #exports::Result<#exports::Option<Self::ActionResult>>{
            match action {
                #(#universal)*
                #(#action_pat)*
                _ => #exports::unreachable!("should be handled by tabs widget")
            }
        }
    }
}
