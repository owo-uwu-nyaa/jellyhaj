mod action;
mod click;
mod init;
mod render;
mod size;
mod text;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Path;

use crate::parse::Container;

pub fn gen_container(container: &Container, exports: &Path) -> TokenStream {
    let ty = &container.ty;
    let cx_constr = &container.cx_constr;

    let init = init::gen_init(container, exports);
    let size = size::gen_size(container, exports);
    let text = text::gen_text(container, exports);
    let action = action::gen_action(container, exports);
    let click = click::gen_click(container, exports);
    let render = render::gen_render(container, exports);
    quote! {
        #[automatically_derived]
        impl<R: #cx_constr> #exports::TabContainer<R> for #ty {
            #init
            #size
            #text
            #action
            #click
            #render
        }
    }
}
