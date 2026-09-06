use proc_macro2::TokenStream;
use quote::quote;
use syn::{Type, parse_quote};

use crate::form::Component;

use super::FieldKind;

impl Component {
    pub fn make_type_assertions(&self) -> impl Iterator<Item = TokenStream> {
        let exports = &self.paths.exports;
        let action_result_ty = &self.action_result;
        let check: Type = parse_quote!(#exports::TypeCheck::<#action_result_ty>);
        self.fields.iter().map(move |field| {
            let ty = &field.ty;
            match field.kind {
                FieldKind::Item { descr: _ } => {
                    quote! {const _:() = #check::is_form_item::<#ty>();}
                }
                FieldKind::Flatten => quote! {const _:() = #check::is_form_component::<#ty>();},
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::{form::tests::example_component, test_helper::assert_tokens_eq};

    #[test]
    fn make_type_assertions() {
        assert_tokens_eq(
            "test-files/form/tokens/type_assertions.rs",
            example_component().make_type_assertions().collect(),
        );
    }
}
