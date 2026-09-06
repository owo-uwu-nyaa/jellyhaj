use proc_macro2::TokenStream;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;

use crate::form::{Component, ShowIf};

impl Component {
    #[must_use]
    pub fn make_show_if_impls(&self) -> TokenStream {
        let exports = &self.paths.exports;
        let data_ty = &self.data;
        let functions =
            self.fields
                .iter()
                .filter_map(|f| f.show_if.as_ref())
                .map(|ShowIf { expr, fun }| {
                    let span = expr.span();
                    quote_spanned! {span=>
                    #[must_use]
                    fn #fun(&self)->#exports::bool{
                        #expr
                    }}
                });
        quote! {
            impl #data_ty {
                #(#functions)*
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{form::tests::example_component, test_helper::assert_tokens_eq};

    #[test]
    fn make_show_if_impls() {
        assert_tokens_eq(
            "test-files/form/tokens/show_if.rs",
            example_component().make_show_if_impls(),
        );
    }
}
