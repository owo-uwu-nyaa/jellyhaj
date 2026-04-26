use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
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
    _sep2: Token![,],
    result_mapper: Type,
}

impl Parse for Args {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Args {
            name: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected form name attribute parameter"))?,
            _sep: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected additional attribute parameter"))?,
            action_result: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected action_result type"))?,
            _sep2: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected additional attribute parameter"))?,
            result_mapper: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected result_mapper type"))?,
        })
    }
}

pub fn parse(args: TokenStream, input: TokenStream) -> Result<ParseResult> {
    let args: Args = parse2(args)?;
    let mut input: ItemStruct = parse2(input)?;
    if let Fields::Named(fields) = &mut input.fields {
        let name = input.ident.to_string();
        let selection_ty = format_ident!("{name}Selection");
        let action_ty = format_ident!("{name}Action");
        let fields: Result<Vec<_>> = fields
            .named
            .iter_mut()
            .filter_map(|field| {
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
                if index
                    .map(|i| {
                        field.attrs.remove(i);
                    })
                    .is_none()
                {
                    Some(field)
                } else {
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
                    Error::new(span, "Every field must have a #[descr(\"\")] attribute")
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
                let enum_id = Ident::new(&name.to_string().to_case(Case::Pascal), name.span());
                Ok(FormItem {
                    name,
                    ty: field.ty.clone(),
                    descr,
                    selection: parse_quote!(#selection_ty::#enum_id),
                    action: parse_quote!(#action_ty::#enum_id),
                    enum_id,
                    show_if,
                    show_if_fun,
                })
            })
            .collect();
        let data_ty = input.ident.clone();
        let size_ident = Ident::new(
            &format!("{}_SIZE", input.ident.to_string().to_case(Case::Constant)),
            input.ident.span(),
        );
        let state_name = Ident::new(&format!("{}State", input.ident), input.ident.span());
        let widget_name = Ident::new(&format!("{}Widget", input.ident), input.ident.span());
        Ok(ParseResult {
            full: input,
            fields: fields?,
            name: args.name,
            action_result: args.action_result,
            data_ty,
            selection_ty,
            action_ty,
            size_ident,
            state_name,
            widget_name,
            result_mapper_ty: args.result_mapper,
        })
    } else {
        Err(Error::new(input.span(), "Struct must have named fields"))
    }
}
