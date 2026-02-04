#[proc_macro_derive(Selection, attributes(descr))]
pub fn selection(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    jellyhaj_form_derive_impl::selection::selection(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn form(
    args: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    jellyhaj_form_derive_impl::form::form(args.into(), input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
