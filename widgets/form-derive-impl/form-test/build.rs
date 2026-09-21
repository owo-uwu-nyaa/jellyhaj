use std::{env, fs, path::PathBuf};

use jellyhaj_form_derive_impl::form::{Component, Form};
use prettyplease::unparse;
use proc_macro2::TokenStream;
use quote::{TokenStreamExt, quote};
use syn::{File, parse2};

fn main() {
    let inner = Component::parse(
        quote! {Quit},
        quote! {
            #[derive(Valuable)]
            pub struct Inner{
            #[form(descr = "checkbox")]
            check: bool,
        }},
    )
    .expect("parsing failed");
    let full = Form::parse(
        quote! {"Test form", Quit, Mapper},
        quote! {
            #[derive(Valuable)]
            pub struct Full{
                #[form(descr = "start", show_if(self.inner.check))]
                start: bool,
                #[form(flatten, show_if(self.start))]
                inner: Inner,
                #[form(flatten)]
                inner2: Inner,
                #[form(descr = "end")]
                end: bool
            }
        },
    )
    .expect("parsing failed");
    let mut file = TokenStream::new();
    file.append_all([inner]);
    file.append_all([full]);
    let file: File = parse2(file).expect("parsing generated code failed");
    let file = unparse(&file);
    let mut out: PathBuf = env::var_os("OUT_DIR").expect("missing OUT_DIR").into();
    fs::create_dir_all(&out).expect("creating out dir");
    out.push("form.rs");
    fs::write(&out, &file).expect("writing out file")
}
