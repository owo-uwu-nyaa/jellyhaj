use proc_macro2::TokenStream;
pub use quote::ToTokens;
use quote::{TokenStreamExt, format_ident, quote};
use syn::{Expr, Ident, ItemStruct, LitStr, Path, Type, parse_quote};

mod action;
mod component;
mod parse;
mod selection;
mod show_if;
mod type_assertions;

struct Paths {
    exports: Path,
    form_component: Path,
    form_data: Path,
    form_item_base: Type,
    with_selection: Path,
    with_selection_mut: Path,
    with_selection_mut_cx: Path,
    with_index_mut: Path,
    with_iter_items: Path,
    with_iter_items_mut: Path,
    with_action_mut: Path,
    form: Path,
}

impl Paths {
    pub fn new(action_result: &Type) -> Self {
        Self {
            exports: parse_quote!(::jellyhaj_form_widget::macro_impl::exports),
            form_component: parse_quote!(::jellyhaj_form_widget::form::component::FormComponent),
            form_data: parse_quote!(::jellyhaj_form_widget::form::FormData),
            form_item_base: parse_quote!(::jellyhaj_form_widget::FormItemBase<#action_result>),
            with_selection: parse_quote!(::jellyhaj_form_widget::form::helpers::WithSelection),
            with_selection_mut: parse_quote!(
                ::jellyhaj_form_widget::form::helpers::WithSelectionMut
            ),
            with_selection_mut_cx: parse_quote!(
                ::jellyhaj_form_widget::form::helpers::WithSelectionMutCX
            ),
            with_index_mut: parse_quote!(::jellyhaj_form_widget::form::helpers::WithIndexMut),
            with_iter_items: parse_quote!(::jellyhaj_form_widget::form::helpers::WithIterItems),
            with_iter_items_mut: parse_quote!(
                ::jellyhaj_form_widget::form::helpers::WithIterItemsMut
            ),
            with_action_mut: parse_quote!(::jellyhaj_form_widget::form::helpers::WithActionMut),
            form: parse_quote!(::jellyhaj_form_widget::form::Form),
        }
    }
}

struct ShowIf {
    expr: Expr,
    fun: Ident,
}

enum FieldKind {
    Item { descr: LitStr },
    Flatten,
}

struct FormField {
    pub name: Ident,
    pub ty: Type,
    pub show_if: Option<ShowIf>,
    pub selection: Path,
    pub action: Path,
    pub enum_id: Ident,
    pub kind: FieldKind,
}

impl FormField {
    const fn is_item(&self) -> bool {
        matches!(self.kind, FieldKind::Item { descr: _ })
    }
    const fn get_descr(&self) -> Option<&LitStr> {
        if let FieldKind::Item { descr } = &self.kind {
            Some(descr)
        } else {
            None
        }
    }
}

pub struct Component {
    fields: Vec<FormField>,
    action_result: Type,
    data: Ident,
    selection: Ident,
    action: Ident,
    original: ItemStruct,
    paths: Paths,
}

impl ToTokens for Component {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        tokens.append_all([&self.original]);
        tokens.append_all(self.make_show_if_impls());
        tokens.append_all(self.make_type_assertions());
        tokens.append_all(self.make_selection_ty());
        tokens.append_all(self.make_selection_default());
        tokens.append_all(self.make_selection_valuable());
        tokens.append_all(self.make_action());
        self.make_impl_component(tokens);
    }
}

pub struct Form {
    name: LitStr,
    result_mapper: Type,
    component: Component,
}

impl Form {
    fn make_form_data_impl(&self) -> TokenStream {
        let data = &self.component.paths.form_data;
        let exports = &self.component.paths.exports;
        let mapper = &self.result_mapper;
        let title = &self.name;
        let ty = &self.component.data;
        let widget_name = format_ident!("{}Widget", &ty);
        let form_wrapper = &self.component.paths.form;
        let vis = &self.component.original.vis;
        quote! {
            impl #data for #ty{
                type Mapper = #mapper;
                const TITLE: &#exports::str = #title;
            }
            #vis type #widget_name = #form_wrapper<#ty>;
        }
    }
}

impl ToTokens for Form {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        self.component.to_tokens(tokens);
        tokens.append_all(self.make_form_data_impl());
    }
}
