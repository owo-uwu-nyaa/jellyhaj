use proc_macro2::{Literal, TokenStream};
use quote::{ToTokens, quote, quote_spanned};
use syn::{
    Expr, Ident, Path, Result, Token, parse::Parse, punctuated::Punctuated, spanned::Spanned,
};

pub struct NodeMapArgs {
    var: Ident,
    _sep: Token![;],
    path: Path,
    _sep2: Token![;],
    exprs: Punctuated<MapEntry, Token![,]>,
}

pub struct MapEntry {
    name: Expr,
    sep: Token![:],
    val: Expr,
}

impl ToTokens for MapEntry {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.name.to_tokens(tokens);
        self.sep.to_tokens(tokens);
        self.val.to_tokens(tokens);
    }
}

impl Parse for MapEntry {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            name: input.parse()?,
            sep: input.parse()?,
            val: input.parse()?,
        })
    }
}

impl Parse for NodeMapArgs {
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

pub fn node_map_impl(input: NodeMapArgs) -> TokenStream {
    let p = &input.path;
    let var = &input.var;
    if input.exprs.is_empty() {
        quote! {let #var = #p::MpvNodeMap::new(&[], &[]);}
    } else if input.exprs.len() > (i32::MAX as usize) {
        quote_spanned! {input.exprs.span()=>compile_error!("Argument list length is greater than i32::MAX.")}
    } else {
        let (ks, vs) =
            <(Vec<Expr>, Vec<Expr>)>::from_iter(input.exprs.into_iter().map(|e| (e.name, e.val)));
        let is: Vec<_> = (0..ks.len())
            .into_iter()
            .map(Literal::usize_unsuffixed)
            .collect();
        quote! {
            let #var = (
                (#(#ks),* ,()),
                (#(#vs),* ,())
            );
            let #var = (
                [#(#p::CStrPtr::new(#var.0.#is)),*],
                [#(#p::ToMpvNode::node(#var.1.#is)),*]
            );
            let #var = #p::MpvNodeMap::new(&#var.0, &#var.1);
        }
    }
}
