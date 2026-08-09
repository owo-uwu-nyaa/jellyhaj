use jellyhaj_form_derive_impl::form::ToTokens;

#[proc_macro_derive(Selection, attributes(descr))]
pub fn selection(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    jellyhaj_form_derive_impl::selection::selection(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn form_widget(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    jellyhaj_form_derive_impl::form::Form::parse(args.into(), input.into()).map_or_else(syn::Error::into_compile_error, ToTokens::into_token_stream)
        .into()
}
#[proc_macro_attribute]
pub fn form_component(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    jellyhaj_form_derive_impl::form::Component::parse(args.into(), input.into()).map_or_else(syn::Error::into_compile_error, ToTokens::into_token_stream)
        .into()
}
