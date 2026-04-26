use proc_macro2::{Literal, Span, TokenStream};
use quote::{quote, quote_spanned};
use syn::{Expr, Ident, ItemStruct, LitStr, Path, Result, Type, parse_quote, spanned::Spanned};

mod parse;

struct FormItem {
    pub name: Ident,
    pub ty: Type,
    pub descr: LitStr,
    pub selection: Path,
    pub action: Path,
    pub enum_id: Ident,
    pub show_if: Option<Expr>,
    pub show_if_fun: Option<Ident>,
}

struct ParseResult {
    fields: Vec<FormItem>,
    name: LitStr,
    action_result: Type,
    data_ty: Ident,
    selection_ty: Ident,
    action_ty: Ident,
    result_mapper_ty: Type,
    full: ItemStruct,
    size_ident: Ident,
    state_name: Ident,
    widget_name: Ident,
}

pub fn form(args: TokenStream, input: TokenStream) -> Result<TokenStream> {
    let ParseResult {
        fields,
        name,
        action_result,
        data_ty,
        selection_ty,
        action_ty,
        full,
        size_ident,
        state_name,
        widget_name,
        result_mapper_ty,
    } = parse::parse(args, input)?;
    let private_mod: Ident = Ident::new(
        &("form_impl_".to_string() + &state_name.to_string()),
        Span::mixed_site(),
    );
    let exports: Path = parse_quote!(::jellyhaj_form_widget::macro_impl::exports);
    let form_item_info_tr: Type =
        parse_quote!(::jellyhaj_form_widget::FormItemInfo<#action_result>);
    let form_data_tr: Path = parse_quote!(::jellyhaj_form_widget::form::FormData);
    let form_data_types_tr: Path = parse_quote!(::jellyhaj_form_widget::form::FormDataTypes);
    let with_selection_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithSelection);
    let with_selection_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithSelectionMut);
    let with_selection_mut_cx_tr: Path =
        parse_quote!(::jellyhaj_form_widget::form::WithSelectionMutCX);
    let with_index_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithIndexMut);
    let with_iter_items_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithIterItems);
    let with_iter_items_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithIterItemsMut);
    let with_action_mut_tr: Path = parse_quote!(::jellyhaj_form_widget::form::WithActionMut);
    let form: Path = parse_quote!(::jellyhaj_form_widget::form::Form);
    let total_size = Literal::usize_suffixed(fields.len());
    let show_if_fns = fields.iter().filter_map(|item| {
        if let (Some(name), Some(expr)) = (item.show_if_fun.as_ref(), item.show_if.as_ref()) {
            let span = expr.span();
            Some(quote_spanned! {span=> pub fn #name(&self)->#exports::bool{
                #expr
            }})
        } else {
            None
        }
    });
    let selection_items = fields.iter().map(|item| {
        let name = &item.enum_id;
        let ty = &item.ty;
        quote! {#name(<#ty as #form_item_info_tr>::SelectionInner)}
    });

    let selection_variant_defs = fields.iter().map(|item| {
        let name = LitStr::new(&item.enum_id.to_string(), item.enum_id.span());
        quote! {#exports::VariantDef::new(#name, #exports::Fields::Unnamed(1))}
    });

    let selection_variant_pats = fields.iter().enumerate().map(|(i, item)| {
        let sel = &item.selection;
        let i = Literal::usize_suffixed(i);
        quote! {super::#sel(_) => #exports::Variant::Static(&DEFS[#i])}
    });

    let selection_value_pats = fields.iter().map(|item| {
        let sel = &item.selection;
        quote! {super::#sel(v) => #exports::Valuable::as_value(v)}
    });

    let action_items = fields.iter().map(|item| {
        let name = &item.enum_id;
        let ty = &item.ty;
        quote! {#name(<#ty as #form_item_info_tr>::Action)}
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
    let with_selection_mut_cx_pats = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let ty = &item.ty;
        let sel = &item.selection;
        let ac = &item.action;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            #sel(s) => W::with_mut::<#index, #ty>(
                with, s, cx.wrap_with(#ac), &mut state.#name, #descr
            )
        }
    });
    let with_index_mut_pats = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let sel = &item.selection;
        let ac = &item.action;
        let ty = &item.ty;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            #index => {
                *this = #sel(W::with_mut::<#index, #ty>(
                    with, cx.wrap_with(#ac), &mut state.#name, #descr
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
        let ac = &item.action;
        let descr = &item.descr;
        let index = Literal::usize_suffixed(i);
        quote! {
            W::with_mut::<#index, #ty>(
                with, cx.wrap_with(#ac), &mut state.#name, #descr
            )?;
        }
    });
    let with_action_mut_pats = fields.iter().enumerate().map(|(i, item)| {
        let name = &item.name;
        let ty = &item.ty;
        let ac = &item.action;
        let index = Literal::usize_suffixed(i);
        quote! {
            #ac(action) =>  W::with_mut::<#index, #ty>(
                with, action, cx.wrap_with(#ac), &mut state.#name
            )
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
        impl #data_ty {
            #(#show_if_fns)*
        }
        #[derive(Debug)]
        #vis enum #selection_ty {
            #(#selection_items),*
        }

        mod #private_mod{

            static DEFS: &[#exports::VariantDef] = &[
                #(#selection_variant_defs),*
            ];

            impl #exports::Valuable for super::#selection_ty{
                fn as_value(&self) -> #exports::Value<'_>{
                    #exports::Value::Enumerable(self)
                }
                fn visit(&self, visit: &mut dyn #exports::Visit){
                    let val = match self{
                        #(#selection_value_pats),*
                    };
                    visit.visit_unnamed_fields(&[val])
                }
            }

            impl #exports::Enumerable for super::#selection_ty{
                fn definition(&self) -> #exports::EnumDef<'_>{
                    #exports::EnumDef::new_static(
                        #name, DEFS
                    )
                }
                fn variant(&self) -> #exports::Variant<'_>{
                    match self{
                        #(#selection_variant_pats),*
                    }
                }
            }
        }

        #[derive(Debug)]
        #vis enum #action_ty {
            #(#action_items),*
        }
        #vis const #size_ident: #exports::usize = #total_size;

        impl #form_data_types_tr for #data_ty{
            type Selector = #selection_ty;
            type AR = #action_result;
            type Action = #action_ty;
            type Mapper = #result_mapper_ty;
        }

        impl #form_data_tr<#total_size> for #data_ty{
            const TITLE: &#exports::str = #name;

            fn with_selection<R: 'static, T, W: #with_selection_tr<R, Self::AR, T>>(
                this: &Self::Selector,
                state: &Self,
                with: W,
            ) -> T {
                match this {
                    #(#with_selection_pats),*
                }
            }

            fn with_selection_mut<R: 'static, T, W: #with_selection_mut_tr<R, Self::AR, T>>(
                this: &mut Self::Selector,
                state: &mut Self,
                with: W,
            ) -> T {
                match this {
                    #(#with_selection_mut_pats),*
                }
            }

            fn with_selection_mut_cx<R: 'static, T, W: #with_selection_mut_cx_tr<R, Self::AR, T>>(
                this: &mut Self::Selector,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                state: &mut Self,
                with: W,
            ) -> T {
                match this {
                    #(#with_selection_mut_cx_pats),*
                }
            }

            fn with_index_mut<R: 'static, W: #with_index_mut_tr<R, Self::AR>>(
                this: &mut Self::Selector,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                state: &mut Self,
                index: #exports::usize,
                with: W,
            ) -> #exports::Result<()>{
                match index {
                    #(#with_index_mut_pats)*
                    v => {#exports::panic!("{v} is out of bounds.")}
                }
                #exports::Result::Ok(())
            }

            fn with_iter<R: 'static, W: #with_iter_items_tr<R, Self::AR>>(
                state: &Self,
                with: &mut W
            ) -> #exports::Result<()>{
                #(#with_iter_items)*
                #exports::Result::Ok(())
            }

            fn with_iter_mut<R: 'static, W: #with_iter_items_mut_tr<R, Self::AR>>(
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                state: &mut Self,
                with: &mut W,
            ) -> #exports::Result<()>{
                #(#with_iter_items_mut)*
                #exports::Result::Ok(())
            }

            fn with_action_mut<R: 'static, T, W: #with_action_mut_tr<R, Self::AR, T>>(
                action: Self::Action,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                state: &mut Self,
                with: W,
            ) -> T {
                match action {
                    #(#with_action_mut_pats),*
                }
            }

            fn show_if(state: &Self) -> [#exports::bool; #total_size]{
                [#(#show_if_items),*]
            }
            fn index(sel: &Self::Selector) -> #exports::usize {
                match sel {#(#index_pats),*}
            }

        }
        #vis type #widget_name = #form<{#total_size}, #data_ty>;
    })
}
