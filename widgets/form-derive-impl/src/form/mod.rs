use proc_macro2::{Literal, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Expr, Ident, ItemStruct, LitStr, Path, Result, Type, parse_quote, spanned::Spanned};

mod parse;

struct FormItem {
    pub name: Ident,
    pub ty: Type,
    pub descr: LitStr,
    pub selection: Path,
    pub selection_id: Ident,
    pub show_if: Option<Expr>,
    pub show_if_fun: Option<Ident>,
}

struct ParseResult {
    fields: Vec<FormItem>,
    name: LitStr,
    action_result: Type,
    state_ty: Ident,
    selection_ty: Ident,
    full: ItemStruct,
}

pub fn form(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let ParseResult {
        fields,
        name,
        action_result,
        state_ty,
        selection_ty,
        full,
    } = parse::parse(args, input)?;
    let form_item_tr: Type = parse_quote!(::jellyhaj_form_widget::FormItem<#action_result>);
    let form_state_tr: Path = parse_quote!(::jellyhaj_form_widget::form::FormData);
    let with_selection_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithSelection);
    let with_selection_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithSelectionMut);
    let with_index_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithIndexMut);
    let with_iter_items_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithIterItems);
    let with_iter_items_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithIterItemsMut);
    let total_size = Literal::usize_suffixed(fields.len());
    let show_if_fns = fields.iter().filter_map(|item| {
        if let (Some(name), Some(expr)) = (item.show_if_fun.as_ref(), item.show_if.as_ref()) {
            let span = expr.span();
            Some(quote_spanned! {span=> pub fn #name(&self)->bool{
                #expr
            }})
        } else {
            None
        }
    });
    let selection_items = fields.iter().map(|item| {
        let name = &item.selection_id;
        let ty = &item.ty;
        quote! {#name(<#ty as #form_item_tr>::SelectionInner)}
    });
    let with_selection_pats = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let ty = &item.ty;
        let sel = &item.selection;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            #sel(s) => W::with::<#index, #ty >(
                with, s, &state.#name, #descr
            )
        }
    });
    let with_selection_mut_pats = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let ty = &item.ty;
        let sel = &item.selection;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            #sel(s) => W::with_mut::<#index, #ty>(
                with, s, &mut state.#name, #descr
            )
        }
    });
    let with_index_mut_pats = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let sel = &item.selection;
        let ty = &item.ty;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            #index => {
                *this = #sel(W::with_mut::<#index, #ty>(
                    with, &mut state.#name, #descr
                )?)
            }
        }
    });
    let with_iter_items = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let ty = &item.ty;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            W::with::<#index, #ty>(
                with, &state.#name, #descr
            )?;
        }
    });
    let with_iter_items_mut = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let ty = &item.ty;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            W::with_mut::<#index, #ty>(
                with, &mut state.#name, #descr
            )?;
        }
    });
    let show_if_items = fields.iter().map(|item| {
        if let Some(show_if) = item.show_if_fun.as_ref() {
            quote! {state.#show_if()}
        } else {
            quote! {true}
        }
    });
    let index_pats = fields.iter().enumerate().map(|(i, item)| {
        let pat = &item.selection;
        let index = Literal::usize_suffixed(i);
        quote! {#pat(_) => #index }
    });
    let vis = full.vis.clone();
    Ok(quote! {
        #full
        impl #state_ty {
            #(#show_if_fns)*
        }
        #[derive(Debug)]
        #vis enum #selection_ty {
            #(#selection_items),*
        }
        impl #form_state_tr<#total_size> for #state_ty{
            type Selector = #selection_ty;
            type AR = #action_result;
            const TITLE: &str = #name;

            fn with_selection<T, W: #with_selection_tr<Self::AR, T>>(
                this: &Self::Selector,
                state: &Self,
                with: W,
            ) -> T {
                match this {
                    #(#with_selection_pats),*
                }
            }

            fn with_mut_selection<T, W: #with_selection_mut_tr<Self::AR, T>>(
                this: &mut Self::Selector,
                state: &mut Self,
                with: W,
            ) -> T {
                match this {
                    #(#with_selection_mut_pats),*
                }
            }

            fn with_index_mut<W: #with_index_mut_tr<Self::AR>>(
                this: &mut Self::Selector,
                state: &mut Self,
                index: usize,
                with: W,
            ) -> Result<()>{
                match index {
                    #(#with_index_mut_pats)*
                    v => {panic!("{v} is out of bounds.")}
                }
                Result::Ok(())
            }

            fn with_iter<W: #with_iter_items_tr<Self::AR>>(
                state: &Self,
                with: &mut W
            ) -> Result<()>{
                #(#with_iter_items)*
                Result::Ok(())
            }
            fn with_iter_mut<W: #with_iter_items_mut_tr<Self::AR>>(
                state: &mut Self,
                with: &mut W,
            ) -> Result<()>{
                #(#with_iter_items_mut)*
                Result::Ok(())
            }
            fn show_if(state: &Self) -> [bool; #total_size]{
                [#(#show_if_items),*]
            }
            fn index(sel: &Self::Selector) -> usize{match sel {#(#index_pats),*}}

        }

    })
}
