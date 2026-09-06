use convert_case::{Case, Casing};
use proc_macro2::TokenStream;
use quote::format_ident;

use crate::form::{Component, FieldKind, Form, FormField, Paths, ShowIf};

use syn::{
    Attribute, Error, Expr, Field, Fields, Ident, ItemStruct, LitStr, Result, Token, Type,
    meta::ParseNestedMeta,
    parenthesized,
    parse::{Parse, ParseBuffer},
    parse_quote, parse2,
    spanned::Spanned,
};

struct FormComponentArgs {
    action_result: Type,
}

impl Parse for FormComponentArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            action_result: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected action_result type"))?,
        })
    }
}

struct FormArgs {
    name: LitStr,
    _sep: Token![,],
    component: FormComponentArgs,
    _sep2: Token![,],
    result_mapper: Type,
}

impl Parse for FormArgs {
    fn parse(input: syn::parse::ParseStream) -> Result<Self> {
        Ok(Self {
            name: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected form name attribute parameter"))?,
            _sep: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected additional attribute parameter"))?,
            component: input.parse()?,
            _sep2: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected additional attribute parameter"))?,
            result_mapper: input
                .parse()
                .map_err(|e| Error::new(e.span(), "Expected result_mapper type"))?,
        })
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
                "every attribute inside a form needs to be annoteted with one of `#[form(skip)]`, `#[form(descr = \"\")]` or `#[form(flatten)]`.",
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

fn parse_component(args: FormComponentArgs, mut original: ItemStruct) -> Result<Component> {
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
        Ok(Component {
            fields,
            action_result: args.action_result,
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
