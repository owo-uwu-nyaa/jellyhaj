use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_assertions(container: &Container, exports: &Path) -> TokenStream {
    let result_ty = &container.result_ty;
    let types = container.tabs.iter().map(|v| &v.ty);
    let common_action = container.common_action.iter();
    let common_target_types = container
        .common_action
        .iter()
        .flat_map(|_| container.tabs.iter().map(|v| &v.ty));
    quote! {
        const _:() = {
            fn assert_widget<W: #exports::JellyhajWidgetBase>() {}
            fn assert_result_ty<T: #exports::Into::<#result_ty>>() {}

            #(fn assert_common<T>() where #common_action: #exports::Into::<Option<T>> {})*

            fn assert_all(){
                #(
                    assert_widget::<#types>();
                    assert_result_ty::<<#types as #exports::JellyhajWidgetBase>::ActionResult>();
                )*

                #(assert_common::<<#common_target_types as #exports::JellyhajWidgetBase>::Action>();)*

            }
        };
    }
}
