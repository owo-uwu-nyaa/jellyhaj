mod assert;
mod container;
mod enum_ty;
mod parse;
mod tabbed;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Error, Path, Result, parse_quote};

fn generate(input: proc_macro::TokenStream) -> Result<TokenStream> {
    let container = parse::parse(input)?;
    let exports: Path = parse_quote!(::jellyhaj_tabs_widget::macro_exports);

    let assertions = assert::gen_assertions(&container, &exports);
    let enum_ty = enum_ty::gen_enum(&container, &exports);
    let tabbed = tabbed::gen_tabbed(&container, &exports);
    let container = container::gen_container(&container, &exports);

    Ok(quote! {
        #assertions

        #enum_ty

        #tabbed
        #container

    })
}

#[proc_macro_derive(TabContainer, attributes(tab))]
pub fn tab_container(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    generate(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
