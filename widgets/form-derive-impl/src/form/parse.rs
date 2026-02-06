use std::char::ToUppercase;

use proc_macro2::{Literal, Span, TokenStream};
use quote::format_ident;

use crate::form::{FormItem, ParseResult};

use syn::{
    Error, Expr, Fields, Ident, ItemStruct, LitStr, Result, Token, Type, parse::Parse, parse_quote,
    parse2, spanned::Spanned,
};

struct Args {
    name: LitStr,
    _sep: Token![,],
    action_result: Type,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Args {
            name: input.parse()?,
            _sep: input.parse()?,
            action_result: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected action_result type"))?,
        })
    }
}

enum Upper {
    None,
    Unchanged(char),
    Changed(ToUppercase),
}

impl Iterator for Upper {
    type Item = char;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Upper::None => None,
            Upper::Unchanged(c) => {
                let c = *c;
                *self = Upper::None;
                Some(c)
            }
            Upper::Changed(to_uppercase) => to_uppercase.next(),
        }
    }
}

fn camel_case(snake: &str) -> String {
    let mut up = true;
    snake
        .chars()
        .flat_map(|c| {
            if c == '_' {
                up = true;
                Upper::None
            } else if up {
                up = false;
                Upper::Changed(c.to_uppercase())
            } else {
                Upper::Unchanged(c)
            }
        })
        .collect()
}

pub fn parse(args: TokenStream, input: TokenStream) -> Result<ParseResult> {
    let args: Args = parse2(args)?;
    let mut input: ItemStruct = parse2(input)?;
    if let Fields::Named(fields) = &mut input.fields {
        let name = input.ident.to_string();
        let selection_ty = format_ident!("{name}Selection");
        let fields_len = Literal::usize_unsuffixed(fields.named.len());
        let height_store_ty = parse_quote!([(u16, bool);#fields_len]);
        let widget_ty = format_ident!("{name}Widget");
        let fields: Result<Vec<_>> = fields
            .named
            .iter_mut().filter_map(|field|{
                let index = field.attrs.iter().enumerate().find_map(|(i, a)| {
                    if a.path()
                        .get_ident()
                        .map(|i| i.to_string().as_str() == "skip")
                        .unwrap_or(false)
                    {
                        Some(i)
                    } else {
                        None
                    }
                });
                if index.map(|i|{
                    field.attrs.remove(i);
                }).is_none(){
                    Some(field)
                }else{
                    None
                }
            })
            .map(|field| {
                let span = field.span();
                let name = field
                    .ident
                    .clone()
                    .ok_or_else(|| Error::new(span, "field has no name"))?;
                let attr_index = field.attrs.iter().enumerate().find_map(|(i, a)| {
                    if a.path()
                        .get_ident()
                        .map(|i| i.to_string().as_str() == "descr")
                        .unwrap_or(false)
                    {
                        Some(i)
                    } else {
                        None
                    }
                });
                let attr = attr_index.map(|i| field.attrs.remove(i)).ok_or_else(|| {
                    Error::new(
                        span,
                        "Every field must have a #[descr(\"\")] attribute",
                    )
                })?;
                let descr: LitStr = attr.parse_args()?;
                let show_if_index = field.attrs.iter().enumerate().find_map(|(i, a)| {
                    if a.path()
                        .get_ident()
                        .map(|i| i.to_string().as_str() == "show_if")
                        .unwrap_or(false)
                    {
                        Some(i)
                    } else {
                        None
                    }
                });
                let show_if = show_if_index
                    .map(|i| field.attrs.remove(i))
                    .map(|e| e.parse_args::<Expr>())
                    .transpose()?;
                let show_if_fun = show_if.as_ref().map(|_| {
                    let name = name.to_string();
                    format_ident!("_show_if_{name}")
                });
                let selection_id = Ident::new(&camel_case(&name.to_string()), Span::call_site());
                Ok(FormItem {
                    name,
                    ty: field.ty.clone(),
                    descr,
                    selection: parse_quote!(#selection_ty::#selection_id),
                    selection_id,
                    show_if,
                    show_if_fun,
                })
            })
            .collect();
        let state_ty = input.ident.clone();
        Ok(ParseResult {
            full: input,
            fields: fields?,
            name: args.name,
            action_result: args.action_result,
            state_ty,
            selection_ty,
            widget_ty,
            height_store_ty,
        })
    } else {
        Err(Error::new(input.span(), "Struct must have named fields"))
    }
}
