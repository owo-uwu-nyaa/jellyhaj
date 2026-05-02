use proc_macro2::{Literal, TokenStream};
use quote::quote;
use syn::{
    AttrStyle, Data, DataEnum, DeriveInput, Error, Fields, Ident, LitStr, Result, Variant, parse2,
    spanned::Spanned,
};

struct SelectionVariant {
    ident: Ident,
    descr: LitStr,
}

fn map_var(var: &Variant) -> Result<SelectionVariant> {
    if var.fields != Fields::Unit {
        Err(Error::new(
            var.span(),
            "This derive macro only supports Unit variants",
        ))
    } else if let Some(attr) = var.attrs.iter().find(|a| {
        a.path()
            .get_ident()
            .is_some_and(|i| i.to_string().as_str() == "descr")
    }) {
        if attr.style == AttrStyle::Outer {
            let arg: LitStr = attr.parse_args()?;
            let descr = arg.value();
            if descr.is_empty() {
                Err(Error::new(arg.span(), "#[descr(\"\")] must not be empty"))
            } else {
                Ok(SelectionVariant {
                    ident: var.ident.clone(),
                    descr: arg,
                })
            }
        } else {
            Err(Error::new(
                attr.span(),
                "#[descr(\"\")] must be an outer attribute",
            ))
        }
    } else {
        Err(Error::new(
            var.span(),
            "Each variant needs a #[descr(\"\")] attribute",
        ))
    }
}

fn descr(vars: &[SelectionVariant], t: &Ident) -> TokenStream {
    let patterns = vars.iter().map(|var| {
        let pattern = &var.ident;
        let descr = &var.descr;
        quote! {#t::#pattern => #descr}
    });
    quote! {
        fn descr(self)->&'static str{
            match self{
                #(#patterns),*
            }
        }
    }
}

fn index(vars: &[SelectionVariant], t: &Ident) -> TokenStream {
    let patterns = vars.iter().enumerate().map(|(i, var)| {
        let pattern = &var.ident;
        let index = Literal::usize_suffixed(i);
        quote! {#t::#pattern => #index}
    });
    quote! {
        fn index(self)->usize{
            match self{
                #(#patterns),*
            }
        }
    }
}

fn max_len(vars: &[SelectionVariant]) -> Result<Literal> {
    let len = if let Some((lit, longest)) = vars
        .iter()
        .map(|v| (&v.descr, v.descr.value()))
        .max_by_key(|v| v.1.len())
    {
        u16::try_from(longest.len()).map_err(|_| {
            Error::new_spanned(
                lit,
                "Selection description is to long, conversion to u16 overflows",
            )
        })?
    } else {
        0
    };
    Ok(Literal::u16_suffixed(len))
}

fn all(vars: &[SelectionVariant], t: &Ident) -> TokenStream {
    let items = vars.iter().map(|var| {
        let pattern = &var.ident;
        quote! {#t::#pattern}
    });
    quote! { &[ #(#items),* ]}
}

fn all_len(vars: &[SelectionVariant], input: &DataEnum) -> Result<Literal> {
    Ok(Literal::u16_unsuffixed(
        vars.len()
            .try_into()
            .map_err(|_| Error::new_spanned(&input.variants, ""))?,
    ))
}

pub fn selection(input: TokenStream) -> Result<TokenStream> {
    let input: DeriveInput = parse2(input)?;

    match input.data {
        Data::Struct(_) | Data::Union(_) => Err(Error::new(
            input.span(),
            "This derive macro only supports Enums",
        )),
        Data::Enum(e) => {
            let variants: Result<Vec<_>> = e.variants.iter().map(map_var).collect();
            let variants = variants?;
            if variants.is_empty() {
                return Err(Error::new(
                    input.ident.span(),
                    "This derive macro requires at least one Variant",
                ));
            }
            let name = &input.ident;
            let descr = descr(&variants, name);
            let index = index(&variants, name);
            let max_len = max_len(&variants)?;
            let all = all(&variants, name);
            let all_len = all_len(&variants, &e)?;
            Ok(quote! {
                impl ::jellyhaj_form_widget::selection::Selection for #name {
                    #descr
                    #index
                    const MAX_LEN: u16 = #max_len;
                    const ALL: &[Self] = #all;
                    const ALL_LEN: u16 = #all_len;
                }
            })
        }
    }
}
