use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{Expr, Ident, Path, Result, Token, parse::Parse, punctuated::Punctuated};

pub struct NodeListArgs {
    var: Ident,
    _sep: Token![;],
    path: Path,
    _sep2: Token![;],
    exprs: Punctuated<Expr, Token![,]>,
}

impl Parse for NodeListArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            var: input.parse()?,
            _sep: input.parse()?,
            path: input.parse()?,
            _sep2: input.parse()?,
            exprs: Punctuated::parse_terminated(input)?,
        })
    }
}

pub fn node_list_impl(input: NodeListArgs) -> TokenStream {
    let p = &input.path;
    let var = &input.var;
    if input.exprs.is_empty() {
        quote! {let #var = #p::MpvNodeList::new(&[]);}
    } else {
        let exprs = input.exprs.into_iter();
        let is: Vec<_> = (0..exprs.len())
            .into_iter()
            .map(Literal::usize_unsuffixed)
            .collect();
        quote! {
            let #var = (#(#exprs),* ,());
            let #var = [#(#p::ToMpvNode::node(#var.#is)),*];
            let #var = #p::MpvNodeList::new(&#var);
        }
    }
}
