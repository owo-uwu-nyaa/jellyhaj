use syn::{
    AttrStyle, Data, DataEnum, DataUnion, DeriveInput, Error, Expr, ExprLit, Field, Ident, Lit,
    LitStr, Path, Result, Token, Type, TypeParamBound, Visibility,
    parse::{End, Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
    spanned::Spanned,
    token::{Enum, Union},
};

pub struct Tab {
    pub ty: Type,
    pub var: Ident,
    pub en_id: Ident,
    pub en_pat: Path,
    pub name: LitStr,
}

pub struct Container {
    pub ty: Ident,
    pub en_ty: Ident,
    pub tabs: Vec<Tab>,
    pub vis: Visibility,
    pub result_ty: Type,
    pub common_action: Option<Type>,
    pub cx_constr: Punctuated<TypeParamBound, Token![+]>,
}

fn to_pascal(snake: &str) -> String {
    let mut pascal = String::new();
    let mut upper = true;
    for c in snake.chars() {
        if c == '_' {
            upper = true;
        } else if upper {
            upper = false;
            pascal.push(c.to_ascii_uppercase());
        } else {
            pascal.push(c);
        }
    }
    pascal
}

struct Args {
    result_ty: Type,
    common_action: Option<Type>,
    cx_constr: Punctuated<TypeParamBound, Token![+]>,
}

impl Parse for Args {
    fn parse(input: ParseStream) -> Result<Self> {
        let name: Ident = input.parse()?;
        if name != "action_result" {
            return Err(Error::new(name.span(), "expected `action_result=Type`"));
        }
        input.parse::<Token![=]>()?;
        let result_ty = input.parse()?;
        let mut common_action: Option<Type> = None;
        let mut cx_constr = None;
        loop {
            if input.is_empty() || input.peek(Token![,]) && input.peek2(End) {
                input.parse::<Option<Token![,]>>()?;
                break;
            }
            if !(input.peek(Token![,]) && input.peek2(Ident)) {
                return Err(input.error(
                    "expected optional next parameter (either `common_action` or `cx_constr`)",
                ));
            }
            input.parse::<Token![,]>()?;
            let name: Ident = input.parse()?;
            if name == "common_action" {
                input.parse::<Token![=]>()?;
                common_action = Some(input.parse()?);
            } else if name == "cx_constr" {
                input.parse::<Token![=]>()?;
                cx_constr = Some(input.parse_terminated(TypeParamBound::parse, Token![+])?);
            } else {
                return Err(input.error(
                    "expected optional next parameter (either `common_action` or `cx_constr`)",
                ));
            }
        }
        let mut cx_constr = cx_constr.unwrap_or_default();
        cx_constr.push(parse_quote!('static));
        Ok(Self {
            result_ty,
            common_action,
            cx_constr,
        })
    }
}

fn parse_global_args(input: &DeriveInput) -> Result<Args> {
    let attr = input.attrs.iter().find(|a|a.style==AttrStyle::Outer && a.meta.path().is_ident("tab")).ok_or_else(||Error::new_spanned(input, "Every item deriving TabContainer needs to be annotated with `#[tab(action_result=Type)]`"))?;
    attr.parse_args()
}

fn parse_name(field: &Field) -> Result<LitStr> {
    let name_attr = field
        .attrs
        .iter()
        .find(|a| a.style == AttrStyle::Outer && a.meta.path().is_ident("tab"));
    let Expr::Lit(ExprLit {
        lit: Lit::Str(ref name),
        ..
    }) = name_attr
        .ok_or_else(|| Error::new_spanned(field, "requires attribute `#[tab=\"descr\"]`"))?
        .meta
        .require_name_value()?
        .value
    else {
        return Err(Error::new_spanned(
            field,
            "tab attribute expects `#[tab=\"descr\"]`",
        ));
    };
    Ok(name.clone())
}

fn parse_member(field: &Field, en_ty: &Ident) -> Result<Tab> {
    let var = field
        .ident
        .as_ref()
        .ok_or_else(|| Error::new_spanned(field, "all fields must be named"));
    let (var, name) = merge_err(var, parse_name(field))?;
    let en_id = Ident::new(&to_pascal(&var.to_string()), field.span());
    let en_con = parse_quote!(#en_ty::#en_id);
    Ok(Tab {
        ty: field.ty.clone(),
        var: var.clone(),
        en_id,
        en_pat: en_con,
        name,
    })
}

fn merge_err<T1, T2>(v1: Result<T1>, v2: Result<T2>) -> Result<(T1, T2)> {
    match (v1, v2) {
        (Ok(v1), Ok(v2)) => Ok((v1, v2)),
        (Err(mut v1), Err(v2)) => {
            v1.combine(v2);
            Err(v1)
        }
        (Err(e), Ok(_)) | (Ok(_), Err(e)) => Err(e),
    }
}

fn combine_err<T>(mut i: impl Iterator<Item = Result<T>>) -> Result<Vec<T>> {
    let mut acc = 'err: {
        let mut acc = Vec::with_capacity(i.size_hint().0);
        for v in i.by_ref() {
            match v {
                Ok(v) => acc.push(v),
                Err(e) => break 'err e,
            }
        }
        return Ok(acc);
    };
    for v in i {
        if let Err(e) = v {
            acc.combine(e);
        }
    }
    Err(acc)
}

pub fn parse(input: proc_macro::TokenStream) -> Result<Container> {
    let input: DeriveInput = syn::parse(input)?;

    let args = parse_global_args(&input);
    match input.data {
        Data::Enum(DataEnum {
            enum_token: Enum { span },
            ..
        })
        | Data::Union(DataUnion {
            union_token: Union { span },
            ..
        }) => Err(Error::new(span, "This derive macro only supports structs")),
        Data::Struct(data_struct) => {
            let en_ty = Ident::new(&format!("{}Action", input.ident), input.ident.span());
            let generics = if input.generics.params.is_empty() {
                Ok(())
            } else {
                Err(Error::new_spanned(
                    &input.generics.params,
                    "This derive macro does not support generics",
                ))
            };
            let tabs = merge_err(
                combine_err(data_struct.fields.iter().map(|f| parse_member(f, &en_ty))),
                generics,
            );
            let ((tabs, ()), args) = merge_err(tabs, args)?;
            Ok(Container {
                ty: input.ident,
                en_ty,
                tabs,
                vis: input.vis,
                result_ty: args.result_ty,
                common_action: args.common_action,
                cx_constr: args.cx_constr,
            })
        }
    }
}
