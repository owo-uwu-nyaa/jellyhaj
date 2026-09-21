use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::format_ident;

use crate::form::{Component, FieldKind, Form, FormField, Paths, ShowIf};

use syn::{
    Attribute, Error, Expr, Field, Fields, Ident, ItemStruct, LitStr, Result, Token, Type,
    TypeParamBound,
    meta::ParseNestedMeta,
    parenthesized,
    parse::{Parse, ParseBuffer},
    parse_quote, parse2,
    punctuated::Punctuated,
    spanned::Spanned,
};

struct FormComponentArgs {
    action_result: Type,
    cx_bounds: Punctuated<TypeParamBound, Token![+]>,
}

fn parse_cx_bound(input: syn::parse::ParseStream) -> Result<TypeParamBound> {
    Ok(TypeParamBound::Trait(input.parse()?))
}

impl Parse for FormComponentArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let action_result = input
            .parse()
            .map_err(|e| Error::new(e.span(), "Expected action_result type"));

        if input.peek(Token![,]) {
            let _: Token![,] = input.parse().expect("just checked");
            let cx_bounds = input
                .parse_terminated(parse_cx_bound, Token![+])
                .map_err(|e| Error::new(e.span(), "Expected additoinal context bounds"));
            let (action_result, cx_bounds) = action_result.combine(cx_bounds)?;
            Ok(Self {
                action_result,
                cx_bounds,
            })
        } else {
            Ok(Self {
                action_result: action_result?,
                cx_bounds: Punctuated::new(),
            })
        }
    }
}

struct FormArgs {
    name: LitStr,
    component: FormComponentArgs,
    result_mapper: Type,
}

impl Parse for FormArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        let name = input
            .parse()
            .map_err(|e| Error::new(e.span(), "Expected form name attribute parameter"));
        let sep: Result<Token![,]> = input
            .parse()
            .map_err(|e| Error::new(e.span(), "Expected additional attribute parameter"));
        if sep.is_err() {
            name.combine(sep)?;
            unreachable!()
        }
        let action_result = input
            .parse()
            .map_err(|e| Error::new(e.span(), "Expected action_result type"));
        let sep: Result<Token![,]> = input
            .parse()
            .map_err(|e| Error::new(e.span(), "Expected additional attribute parameter"));
        if sep.is_err() {
            name.combine(action_result).combine(sep)?;
            unreachable!()
        }
        let result_mapper = input
            .parse()
            .map_err(|e| Error::new(e.span(), "Expected result_mapper type"));
        if input.peek(Token![,]) {
            let _: Token![,] = input.parse().expect("just checked");
            let cx_bounds = input
                .parse_terminated(parse_cx_bound, Token![+])
                .map_err(|e| Error::new(e.span(), "Expected additoinal context bounds"));
            let (((name, action_result), result_mapper), cx_bounds) = name
                .combine(action_result)
                .combine(result_mapper)
                .combine(cx_bounds)?;
            Ok(Self {
                name,
                component: FormComponentArgs {
                    action_result,
                    cx_bounds,
                },
                result_mapper,
            })
        } else {
            let ((name, action_result), result_mapper) =
                name.combine(action_result).combine(result_mapper)?;

            Ok(Self {
                name,
                component: FormComponentArgs {
                    action_result,
                    cx_bounds: Punctuated::new(),
                },
                result_mapper,
            })
        }
    }
}

fn consume_form_attr(
    attrs: &mut Vec<Attribute>,
    mut logic: impl FnMut(ParseNestedMeta) -> Result<()>,
) -> Result<()> {
    attrs
        .extract_if(.., |attr| attr.path().is_ident("form"))
        .try_for_each(|attr| attr.parse_nested_meta(&mut logic))
}

fn parse_show_if(from: &ParseBuffer) -> Result<Expr> {
    let content;
    parenthesized!(content in from);
    content.parse()
}

fn parse_field(
    field: &mut Field,
    selection_ty: &Ident,
    action_ty: &Ident,
) -> Result<Option<FormField>> {
    let name = field
        .ident
        .clone()
        .ok_or_else(|| Error::new_spanned(&field, "field has no name"))?;
    let mut skip = false;
    let mut descr: Option<LitStr> = None;
    let mut show_if: Option<Expr> = None;
    let mut flatten = false;
    let mut errors = vec![];
    let res: Option<FormField> = 'res: {
        if let Err(e) = consume_form_attr(&mut field.attrs, |meta| {
            if meta.path.is_ident("skip") {
                if meta.input.is_empty() {
                    skip = true;
                } else {
                    errors.push(meta.error("`skip` has no parameters"));
                }
            } else if meta.path.is_ident("descr") {
                match meta.value().and_then(ParseBuffer::parse) {
                    Ok(v) => descr = Some(v),
                    Err(e) => errors.push(e),
                }
            } else if meta.path.is_ident("show_if") {
                match parse_show_if(meta.input) {
                    Ok(v) => show_if = Some(v),
                    Err(e) => errors.push(e),
                }
            } else if meta.path.is_ident("flatten") {
                if meta.input.is_empty() || meta.input.peek(Token![,]) {
                    flatten = true;
                } else {
                    errors.push(meta.error("`flatten` has no parameters"));
                }
            } else {
                errors.push(meta.error(
                "unrecognized attribute. valid attributes: `skip`, `descr`, `show_if`, `flatten`",
            ));
            }
            Ok(())
        }) {
            errors.push(e);
        }
        if skip {
            if descr.is_some() || show_if.is_some() || flatten {
                errors.push(Error::new_spanned(
                    field,
                    "`skip` is incompatible with all other #[form] attributes.",
                ));
            }
            break 'res None;
        }

        let show_if = show_if.map(|expr| {
            let name = name.to_string();
            let fun = format_ident!("_show_if_{name}");
            ShowIf { expr, fun }
        });
        let enum_id = Ident::new(&name.to_string().to_case(Case::Pascal), name.span());

        let selection = parse_quote!(#selection_ty::#enum_id);
        let action = parse_quote!(#action_ty::#enum_id);
        if flatten {
            if descr.is_some() {
                errors.push(Error::new_spanned(
                    field,
                    "`flatten` is incompatible with `descr`.",
                ));
                None
            } else {
                Some(FormField {
                    name,
                    ty: field.ty.clone(),
                    show_if,
                    kind: FieldKind::Flatten,
                    selection,
                    action,
                    enum_id,
                })
            }
        } else if let Some(descr) = descr {
            Some(FormField {
                name,
                ty: field.ty.clone(),
                show_if,
                kind: FieldKind::Item { descr },
                selection,
                action,
                enum_id,
            })
        } else {
            errors.push(Error::new_spanned(
                field,
                "every attribute inside a form needs to be annotated with one of `#[form(skip)]`, `#[form(descr = \"\")]` or `#[form(flatten)]`.",
            ));
            None
        }
    };
    collect_errors(errors)?;
    Ok(res)
}

fn collect_errors(errors: Vec<Error>) -> Result<()> {
    let mut errors = errors.into_iter();
    if let Some(e) = errors.next() {
        Err(errors.fold(e, |mut e1, e2| {
            e1.combine(e2);
            e1
        }))
    } else {
        Ok(())
    }
}

fn parse_component(mut args: FormComponentArgs, mut original: ItemStruct) -> Result<Component> {
    if let Fields::Named(fields) = &mut original.fields {
        let data = original.ident.clone();
        let selection = format_ident!("{data}Selection");
        let action = format_ident!("{data}Action");
        let mut errors = vec![];
        let fields: Vec<_> = fields
            .named
            .iter_mut()
            .filter_map(|field| parse_field(field, &selection, &action).transpose())
            .filter_map(|f| match f {
                Ok(v) => Some(v),
                Err(e) => {
                    errors.push(e);
                    None
                }
            })
            .collect();
        collect_errors(errors)?;
        let paths = Paths::new(&args.action_result);
        args.cx_bounds.push(parse_quote!('static));
        Ok(Component {
            fields,
            action_result: args.action_result,
            cx_bounds: args.cx_bounds,
            data,
            selection,
            action,
            original,
            paths,
        })
    } else {
        Err(Error::new(original.span(), "Struct must have named fields"))
    }
}

impl Component {
    pub fn parse(args: TokenStream, input: TokenStream) -> Result<Self> {
        let (args, input) = parse2(args).combine(parse2(input))?;
        parse_component(args, input)
    }
}

fn parse_form(args: FormArgs, original: ItemStruct) -> Result<Form> {
    Ok(Form {
        name: args.name,
        result_mapper: args.result_mapper,
        component: parse_component(args.component, original)?,
    })
}

impl Form {
    pub fn parse(args: TokenStream, input: TokenStream) -> Result<Self> {
        let (args, input) = parse2(args).combine(parse2(input))?;
        parse_form(args, input)
    }
}

trait ResExt<T1> {
    fn combine<T2>(self, other: Result<T2>) -> Result<(T1, T2)>;
}
impl<T1> ResExt<T1> for Result<T1> {
    #[allow(clippy::inline_always)]
    #[inline(always)]
    fn combine<T2>(self, other: Result<T2>) -> Result<(T1, T2)> {
        match (self, other) {
            (Ok(v1), Ok(v2)) => Ok((v1, v2)),
            (Ok(_), Err(e)) | (Err(e), Ok(_)) => Err(e),
            (Err(mut e1), Err(e2)) => {
                e1.combine(e2);
                Err(e1)
            }
        }
    }
}
