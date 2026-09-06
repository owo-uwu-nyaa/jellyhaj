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

#[derive(PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
struct ShowIf {
    expr: Expr,
    fun: Ident,
}

#[derive(PartialEq, Eq, Debug)]
enum FieldKind {
    Item { descr: LitStr },
    Flatten,
}

#[derive(PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
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

#[derive(PartialEq, Eq, Debug)]
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

#[cfg(test)]

pub mod tests {
    use quote::{format_ident, quote};
    use syn::{Ident, ItemStruct, Result, Type, parse_quote};

    use crate::form::{Component, FieldKind, Form, FormField, Paths, ShowIf};

    use pretty_assertions::assert_eq;

    pub fn example_component() -> Component {
        let action_result: Type = parse_quote!(crate::ExampleActionResult);
        let original: ItemStruct = parse_quote!(
            pub struct Example {
                #[test_attr]
                simple1: bool,
                simple2: &'static str,
                #[test_attr]
                skip1: (),
                flatten1: Comp1,
                skip2: (),
                #[test_attr]
                flatten2: crate::Comp2,
                simple3: Simple,
            }
        );
        let data = original.ident.clone();
        let paths = Paths::new(&action_result);
        let selection = parse_quote!(ExampleSelection);
        let action = parse_quote!(ExampleAction);
        let fields = vec![
            {
                let enum_id: Ident = parse_quote!(Simple1);
                FormField {
                    name: parse_quote!(simple1),
                    enum_id: enum_id.clone(),
                    ty: parse_quote!(bool),
                    show_if: None,
                    selection: parse_quote!(#selection::#enum_id),
                    action: parse_quote!(#action::#enum_id),
                    kind: FieldKind::Item {
                        descr: parse_quote!("simple 1"),
                    },
                }
            },
            {
                let enum_id: Ident = parse_quote!(Simple2);
                let name: Ident = parse_quote!(simple2);
                FormField {
                    name: name.clone(),
                    enum_id: enum_id.clone(),
                    ty: parse_quote!(&'static str),
                    show_if: Some(ShowIf {
                        expr: parse_quote!(super::test(self.simple1)),
                        fun: format_ident!("_show_if_{name}"),
                    }),
                    selection: parse_quote!(#selection::#enum_id),
                    action: parse_quote!(#action::#enum_id),
                    kind: FieldKind::Item {
                        descr: parse_quote!("simple 2"),
                    },
                }
            },
            {
                let enum_id: Ident = parse_quote!(Flatten1);
                FormField {
                    name: parse_quote!(flatten1),
                    enum_id: enum_id.clone(),
                    ty: parse_quote!(Comp1),
                    show_if: None,
                    selection: parse_quote!(#selection::#enum_id),
                    action: parse_quote!(#action::#enum_id),
                    kind: FieldKind::Flatten,
                }
            },
            {
                let enum_id: Ident = parse_quote!(Flatten2);
                let name: Ident = parse_quote!(flatten2);
                FormField {
                    name: name.clone(),
                    enum_id: enum_id.clone(),
                    ty: parse_quote!(crate::Comp2),
                    show_if: Some(ShowIf {
                        expr: parse_quote!(self.simple1),
                        fun: format_ident!("_show_if_{name}"),
                    }),
                    selection: parse_quote!(#selection::#enum_id),
                    action: parse_quote!(#action::#enum_id),
                    kind: FieldKind::Flatten,
                }
            },
            {
                let enum_id: Ident = parse_quote!(Simple3);
                FormField {
                    name: parse_quote!(simple3),
                    enum_id: enum_id.clone(),
                    ty: parse_quote!(Simple),
                    show_if: None,
                    selection: parse_quote!(#selection::#enum_id),
                    action: parse_quote!(#action::#enum_id),
                    kind: FieldKind::Item {
                        descr: parse_quote!("simple 3"),
                    },
                }
            },
        ];
        Component {
            fields,
            action_result,
            data,
            selection,
            action,
            original,
            paths,
        }
    }

    #[test]
    fn parse_example_component() -> Result<()> {
        let args = quote! {
            crate::ExampleActionResult
        };
        let input = quote! {
            pub struct Example{
                #[form(descr = "simple 1")]
                #[test_attr]
                simple1: bool,
                #[form(descr = "simple 2", show_if(super::test(self.simple1)))]
                simple2: &'static str,
                #[test_attr]
                #[form(skip)]
                skip1 : (),
                #[form(flatten)]
                flatten1 : Comp1,
                #[form(skip)]
                skip2 : (),
                #[test_attr]
                #[form(flatten, show_if(self.simple1))]
                flatten2 : crate::Comp2,
                #[form(descr = "simple 3")]
                simple3: Simple,
            }
        };
        let parsed = Component::parse(args, input)?;
        assert_eq!(example_component(), parsed);
        Ok(())
    }

    pub fn example_form() -> Form {
        Form {
            name: parse_quote!("Example Form"),
            result_mapper: parse_quote!(Mapper),
            component: example_component(),
        }
    }

    #[test]
    fn parse_example_form() {
        let args = quote! {
            "Example Form", crate::ExampleActionResult, Mapper
        };
        let input = quote! {
            pub struct Example{
                #[form(descr = "simple 1")]
                #[test_attr]
                simple1: bool,
                #[form(descr = "simple 2", show_if(super::test(self.simple1)))]
                simple2: &'static str,
                #[test_attr]
                #[form(skip)]
                skip1 : (),
                #[form(flatten)]
                flatten1 : Comp1,
                #[form(skip)]
                skip2 : (),
                #[test_attr]
                #[form(flatten, show_if(self.simple1))]
                flatten2 : crate::Comp2,
                #[form(descr = "simple 3")]
                simple3: Simple,
            }
        };
        assert_eq!(
            example_form(),
            Form::parse(args, input).expect("invalid example form")
        )
    }
}
