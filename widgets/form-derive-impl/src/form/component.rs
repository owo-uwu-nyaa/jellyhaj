use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::{TokenStreamExt, quote, quote_spanned};
use syn::Token;

use crate::form::{Component, FieldKind};

impl Component {
    pub fn make_impl_component_base(&self, tokens: &mut TokenStream) {
        tokens.append_all(quote! {
            #[allow(unused_parens)]
            #[automatically_derived]
        });
        tokens.append_all([<Token![impl]>::default()]);
        tokens.append_all([&self.paths.form_component_base]);
        tokens.append_all([<Token![for]>::default()]);
        tokens.append_all([&self.data]);
        let mut impls = TokenStream::new();
        impls.append_all(self.make_component_type_defs());
        impls.append_all(self.make_with_selection());
        impls.append_all(self.make_with_selection_mut());
        self.append_show_if(&mut impls);
        impls.append_all(self.make_index());
        self.append_total_size(&mut impls);
        self.append_make_selection_default(&mut impls);
        tokens.append(TokenTree::Group(Group::new(Delimiter::Brace, impls)));
    }
    pub fn make_impl_component(&self, tokens: &mut TokenStream) {
        let component = &self.paths.form_component;
        let cx_bounds = &self.cx_bounds;
        tokens.append_all(quote! {
            #[allow(unused_parens)]
            #[automatically_derived]
            impl<R: #cx_bounds> #component<R>
        });
        tokens.append_all([<Token![for]>::default()]);
        tokens.append_all([&self.data]);
        let mut impls = TokenStream::new();
        impls.append_all(self.make_with_selection_mut_cx());
        self.append_with_index_mut(&mut impls);
        impls.append_all(self.make_with_iter());
        impls.append_all(self.make_with_iter_mut());
        impls.append_all(self.make_with_action_mut());
        tokens.append(TokenTree::Group(Group::new(Delimiter::Brace, impls)));
    }

    fn make_component_type_defs(&self) -> TokenStream {
        let selection = &self.selection;
        let action = &self.action;
        let ar = &self.action_result;
        quote! {
            type Selector = #selection;
            type AR = #ar;
            type Action = #action;

        }
    }

    fn make_with_selection(&self) -> TokenStream {
        let with_selection = &self.paths.with_selection;
        let exports = &self.paths.exports;
        let component_base = &self.paths.form_component_base;
        let pats = self.fields.iter().enumerate().map(|(i, item)| {
            let name = &item.name;
            let ty = &item.ty;
            let sel = &item.selection;
            let index = self.make_base_index_expr(i);
            let span = name.span();
            match &item.kind {
                FieldKind::Flatten => {
                    quote_spanned! {span=>
                        #sel(s) => #component_base::with_selection(
                            &self.#name,
                            (base_index + #index),
                            s,
                            with
                        )
                    }
                }
                FieldKind::Item { descr } => {
                    quote_spanned! {span=>
                        #sel(s) => W::with::<#ty>(
                            with,
                            s,
                            &self.#name,
                            #descr,
                            (base_index + #index)
                        )
                    }
                }
            }
        });
        let selection = &self.selection;
        quote! {
            fn with_selection< W: #with_selection<Self::AR>>(
                &self,
                base_index: #exports::usize,
                this: &#selection,
                with: W,
            ) -> #exports::bool {
                match this {
                    #(#pats),*
                }
            }
        }
    }

    fn make_with_selection_mut(&self) -> TokenStream {
        let with_selection_mut = &self.paths.with_selection_mut;
        let exports = &self.paths.exports;
        let component_base = &self.paths.form_component_base;
        let pats = self.fields.iter().enumerate().map(|(i, item)| {
            let name = &item.name;
            let ty = &item.ty;
            let sel = &item.selection;
            let index = self.make_base_index_expr(i);
            let span = name.span();
            match &item.kind {
                FieldKind::Flatten => {
                    quote_spanned! {span=>
                        #sel(s) => #component_base::with_selection_mut(
                            &mut self.#name,
                            (base_index + #index),
                            s,
                            with
                        )
                    }
                }
                FieldKind::Item { descr } => {
                    quote_spanned! {span=>
                        #sel(s) => W::with_mut::<#ty>(
                            with,
                            s,
                            &mut self.#name,
                            #descr,
                            (base_index + #index)
                        )
                    }
                }
            }
        });
        let selection = &self.selection;
        quote! {
            fn with_selection_mut<W: #with_selection_mut<Self::AR>>(
                &mut self,
                base_index: #exports::usize,
                this: &mut #selection,
                with: W,
            ) {
                match this {
                    #(#pats),*
                }
            }
        }
    }

    fn make_with_selection_mut_cx(&self) -> TokenStream {
        let with_selection_mut_cx = &self.paths.with_selection_mut_cx;
        let exports = &self.paths.exports;
        let component = &self.paths.form_component;
        let pats = self.fields.iter().enumerate().map(|(i, item)| {
            let name = &item.name;
            let ty = &item.ty;
            let sel = &item.selection;
            let index = self.make_base_index_expr(i);
            let action = &item.action;
            let span = name.span();
            match &item.kind {
                FieldKind::Flatten => {
                    quote_spanned! {span=>
                        #sel(s) => #component::with_selection_mut_cx(
                            &mut self.#name,
                            (base_index + #index),
                            s,
                            cx.wrap_with(#action),
                            with
                        )
                    }
                }
                FieldKind::Item { descr } => {
                    quote_spanned! {span=>
                        #sel(s) => W::with_mut::<#ty>(
                            with,
                            s,
                            cx.wrap_with(#action),
                            &mut self.#name,
                            #descr,
                            (base_index + #index)
                        )
                    }
                }
            }
        });
        let selection = &self.selection;
        quote! {
            fn with_selection_mut_cx<T: #exports::Default, W: #with_selection_mut_cx<R, Self::AR, T>>(
                &mut self,
                base_index: #exports::usize,
                this: &mut #selection,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                with: W,
            ) -> #exports::Result<T> {
                match this {
                    #(#pats),*
                }
            }
        }
    }

    fn append_with_index_mut(&self, stream: &mut TokenStream) {
        let with_index_mut = &self.paths.with_index_mut;
        let component = &self.paths.form_component;
        let component_base = &self.paths.form_component_base;
        let exports = &self.paths.exports;

        let selection = &self.selection;
        stream.append_all(
        quote! {
            fn with_index_mut<W: #with_index_mut<R, Self::AR>>(
                &mut self,
                mut base_index: #exports::usize,
                this: &mut #selection,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                index: #exports::usize,
                with: W,
            ) -> #exports::Result<()>
        }
        );

        let mut body = TokenStream::new();
        let mut fields = self.fields.iter().peekable();

        loop {
            let mut processed = 0usize;
            let mut match_body = TokenStream::new();
            while let Some(is_item) = fields.peek().map(|f| f.is_item())
                && is_item
            {
                let item = fields.next().expect("just checked");
                let sel = &item.selection;
                let action = &item.action;
                let name = &item.name;
                let descr = item.get_descr().expect("just checked");
                let pat = Literal::usize_suffixed(processed);
                let span = name.span();
                match_body.append_all(quote_spanned! {span=>
                    #pat => {
                        *this = #sel(W::with_mut(
                            with,
                            cx.wrap_with(#action),
                            &mut self.#name,
                            #descr,
                            base_index + #pat
                        )?);
                        return Ok(())
                    }
                });
                processed += 1;
            }
            if !match_body.is_empty() {
                match_body.append_all(quote! {_ => {}});
                body.append_all(quote! {
                    match index - base_index
                });
                body.append(Group::new(Delimiter::Brace, match_body));
            }
            // flatten
            if let Some(item) = fields.next() {
                if processed > 0 {
                    let processed = Literal::usize_suffixed(processed);
                    body.append_all(quote! {base_index += #processed;});
                }
                let name = &item.name;
                let action = &item.action;
                let sel = &item.selection;
                let span = name.span();
                body.append_all(quote_spanned! {span=>
                    let cur = #component_base::total_size(&self.#name);
                    if index < base_index + cur {
                        let mut res = #exports::Default::default();
                        #component::with_index_mut(
                            &mut self.#name,
                            base_index,
                            &mut res,
                            cx.wrap_with(#action),
                            index,
                            with,
                        )?;
                        *this = #sel(res);
                        return Ok(())
                    }else{
                        base_index += cur;
                    }
                });
            } else {
                break;
            }
        }
        body.append_all(quote! {
            #exports::panic!("index out of bounds")
        });

        stream.append(Group::new(Delimiter::Brace, body));
    }

    fn make_with_iter(&self) -> TokenStream {
        let with_iter_items = &self.paths.with_iter_items;
        let component = &self.paths.form_component;
        let component_base = &self.paths.form_component_base;
        let exports = &self.paths.exports;

        let items = self.fields.iter().map(|item| {
            let name = &item.name;
            let span = name.span();
            match &item.kind {
                FieldKind::Item { descr } => {
                    quote_spanned! {span=>
                        W::with(
                            with,
                            &self.#name,
                            #descr,
                            base_index
                        )?;
                        base_index += 1;
                    }
                }
                FieldKind::Flatten => {
                    quote_spanned! {span=>
                        #component::with_iter(
                            &self.#name,
                            base_index,
                            with,
                        )?;
                        base_index += #component_base::total_size(&self.#name);
                    }
                }
            }
        });

        quote! {
            fn with_iter< W: #with_iter_items<R, Self::AR>>(
                &self,
                mut base_index: #exports::usize,
                with: &mut W,
            ) -> #exports::Result<()>{
                #(#items)*
                Ok(())
            }

        }
    }

    fn make_with_iter_mut(&self) -> TokenStream {
        let with_iter_items_mut = &self.paths.with_iter_items_mut;
        let component = &self.paths.form_component;
        let component_base = &self.paths.form_component_base;
        let exports = &self.paths.exports;

        let items = self.fields.iter().map(|item| {
            let name = &item.name;
            let action = &item.action;
            let span = name.span();
            let show = if let Some(fun) = item.show_if.as_ref().map(|v| &v.fun) {
                quote_spanned! {span=>show && self.#fun()}
            } else {
                quote_spanned! {span=>show}
            };
            match &item.kind {
                FieldKind::Item { descr } => {
                    quote_spanned! {span=>
                        {
                            let show = #show;
                            W::with_mut(
                                with,
                                cx.wrap_with(#action),
                                &mut self.#name,
                                #descr,
                                base_index,
                                show,
                            )?;
                            base_index += 1;
                        }
                    }
                }
                FieldKind::Flatten => {
                    quote_spanned! {span=>
                        {
                            let show = #show;
                            #component::with_iter_mut(
                                &mut self.#name,
                                base_index,
                                cx.wrap_with(#action),
                                with,
                                show
                            )?;
                            base_index += #component_base::total_size(&self.#name);
                        }
                    }
                }
            }
        });

        quote! {
            fn with_iter_mut< W: #with_iter_items_mut<R, Self::AR>>(
                &mut self,
                mut base_index: #exports::usize,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                with: &mut W,
                show: bool,
            ) -> #exports::Result<()>{
                #(#items)*
                Ok(())
            }

        }
    }

    fn make_with_action_mut(&self) -> TokenStream {
        let with_action_mut = &self.paths.with_action_mut;
        let component = &self.paths.form_component;
        let exports = &self.paths.exports;

        let pats = self.fields.iter().enumerate().map(|(i, item)| {
            let name = &item.name;
            let action = &item.action;
            let index = self.make_base_index_expr(i);
            let span = name.span();
            match &item.kind {
                FieldKind::Item { descr: _ } => {
                    quote_spanned! {span=>
                        #action(a) => W::with_mut(
                            with,
                            a,
                            cx.wrap_with(#action),
                            &mut self.#name,
                            (base_index + #index),
                        )
                    }
                }
                FieldKind::Flatten => {
                    quote_spanned! {span=>
                        #action(a) => #component::with_action_mut(
                            &mut self.#name,
                            (base_index + #index),
                            a,
                            cx.wrap_with(#action),
                            with
                        )
                    }
                }
            }
        });

        quote! {
            fn with_action_mut<T, W: #with_action_mut<R, Self::AR, T>>(
                &mut self,
                base_index: #exports::usize,
                action: Self::Action,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                with: W,
            ) -> #exports::Result<#exports::Option<T>>{
                match action {
                    #(#pats),*
                }
            }

        }
    }

    fn append_show_if(&self, stream: &mut TokenStream) {
        let component_base = &self.paths.form_component_base;
        let exports = &self.paths.exports;
        stream.append_all(quote! {
            fn show_if(&self, mut index: #exports::usize) -> #exports::bool
        });

        let mut body = TokenStream::new();
        let mut fields = self.fields.iter().peekable();
        let mut always_show = Vec::new();
        loop {
            let mut processed = 0usize;
            let mut match_body = TokenStream::new();
            always_show.clear();
            while let Some(is_item) = fields.peek().map(|f| f.is_item())
                && is_item
            {
                let item = fields.next().expect("just checked");
                if let Some(show_if) = &item.show_if {
                    let pat = Literal::usize_suffixed(processed);
                    let fun = &show_if.fun;
                    match_body.append_all(quote! {
                        #pat => return self.#fun(),
                    });
                } else {
                    always_show.push(processed);
                }
                processed += 1;
            }
            if !always_show.is_empty() {
                match_body.append_separated(
                    always_show.iter().copied().map(Literal::usize_suffixed),
                    <Token![|]>::default(),
                );
                match_body.append_all(quote! {=> return true,});
            }
            if !match_body.is_empty() {
                match_body.append_all(quote! {_ => {}});

                body.append_all(quote! {match index});
                body.append(Group::new(Delimiter::Brace, match_body));
            }
            // flatten
            if let Some(item) = fields.next() {
                if processed > 0 {
                    let processed = Literal::usize_suffixed(processed);
                    body.append_all(quote! {index -= #processed;});
                }
                let name = &item.name;
                let and = if let Some(fun) = item.show_if.as_ref().map(|s| &s.fun) {
                    quote! {self.#fun() &&}
                } else {
                    TokenStream::new()
                };
                body.append_all(quote! {
                    let cur = #component_base::total_size(&self.#name);
                    if index < cur {
                        return #and #component_base::show_if(&self.#name, index);
                    }
                    index -= cur;
                });
            } else {
                break;
            }
        }
        body.append_all(quote! {
           ; #exports::panic!("index out of bounds")
        });

        stream.append(Group::new(Delimiter::Brace, body));
    }

    fn make_index(&self) -> TokenStream {
        let component_base = &self.paths.form_component_base;
        let exports = &self.paths.exports;
        let pats = self.fields.iter().enumerate().map(|(index, item)| {
            let index = self.make_base_index_expr(index);
            let sel = &item.selection;
            if item.is_item() {
                quote! {#sel(_) => #index,}
            } else {
                let name = &item.name;
                quote! {
                    #sel(sel) => {
                        (
                            #index
                            + #component_base::index(&self.#name, sel)
                        )
                    },
                }
            }
        });
        let selection = &self.selection;
        quote! {
            fn index(&self, sel: &#selection) -> #exports::usize{
                match sel {
                    #(#pats)*
                }
            }
        }
    }

    fn append_total_size(&self, stream: &mut TokenStream) {
        let exports = &self.paths.exports;
        stream.append_all(quote! {fn total_size(&self) -> #exports::usize});
        stream.append(Group::new(
            Delimiter::Brace,
            self.make_base_index_expr(self.fields.len()),
        ));
    }

    fn make_base_index_expr(&self, index: usize) -> TokenStream {
        let mut offset = 0usize;
        let component_base = &self.paths.form_component_base;
        let folded = self.fields[0..index].iter().filter_map(|item| {
            if matches!(item.kind, FieldKind::Flatten) {
                let name = &item.name;
                Some(quote! {#component_base::total_size(&self.#name)})
            } else {
                offset += 1;
                None
            }
        });
        let mut res = TokenStream::new();
        res.append_terminated(folded, <Token![+]>::default());
        res.append(Literal::usize_suffixed(offset));
        res
    }

    fn append_make_selection_default(&self, stream: &mut TokenStream) {
        let component_base = &self.paths.form_component_base;
        let exports = &self.paths.exports;

        let selection = &self.selection;
        stream.append_all(quote! {
            fn make_selection_default(
                &self,
                mut base_index: #exports::usize,
                index: #exports::usize,
            ) -> #selection
        });

        let mut body = TokenStream::new();
        let mut fields = self.fields.iter().peekable();

        loop {
            let mut processed = 0usize;
            let mut match_body = TokenStream::new();
            while let Some(is_item) = fields.peek().map(|f| f.is_item())
                && is_item
            {
                let item = fields.next().expect("just checked");
                let sel = &item.selection;
                let pat = Literal::usize_suffixed(processed);
                let span = item.name.span();
                match_body.append_all(quote_spanned! {span=>
                    #pat => {return #sel(#exports::Default::default())}
                });
                processed += 1;
            }
            if !match_body.is_empty() {
                match_body.append_all(quote! {_ => {}});
                body.append_all(quote! {
                    match index - base_index
                });
                body.append(Group::new(Delimiter::Brace, match_body));
            }
            // flatten
            if let Some(item) = fields.next() {
                if processed > 0 {
                    let processed = Literal::usize_suffixed(processed);
                    body.append_all(quote! {base_index += #processed;});
                }
                let name = &item.name;
                let sel = &item.selection;
                let span = item.name.span();
                body.append_all(quote_spanned! {span=>
                    let cur = #component_base::total_size(&self.#name);
                    if index < base_index + cur {
                        return #sel(#component_base::make_selection_default(
                            &self.#name,base_index, index
                        ))
                    }else{
                        base_index += cur;
                    }
                });
            } else {
                break;
            }
        }
        body.append_all(quote! {
            #exports::panic!("index out of bounds")
        });
        stream.append(Group::new(Delimiter::Brace, body));
    }
}

#[cfg(test)]
mod tests {
    use proc_macro2::TokenStream;

    use crate::{form::tests::example_component, test_helper::assert_tokens_eq};

    #[test]
    fn make_component_type_defs() {
        assert_tokens_eq(
            "test-files/form/tokens/comp_type_defs.rs",
            example_component().make_component_type_defs(),
        );
    }
    #[test]
    fn make_with_selection() {
        assert_tokens_eq(
            "test-files/form/tokens/with_selection.rs",
            example_component().make_with_selection(),
        );
    }
    #[test]
    fn make_with_selection_mut() {
        assert_tokens_eq(
            "test-files/form/tokens/with_selection_mut.rs",
            example_component().make_with_selection_mut(),
        );
    }
    #[test]
    fn make_with_selection_mut_cx() {
        assert_tokens_eq(
            "test-files/form/tokens/with_selection_mut_cx.rs",
            example_component().make_with_selection_mut_cx(),
        );
    }
    #[test]
    fn append_with_index_mut() {
        let mut out = TokenStream::new();
        example_component().append_with_index_mut(&mut out);
        assert_tokens_eq("test-files/form/tokens/with_index_mut.rs", out);
    }
    #[test]
    fn make_with_iter() {
        assert_tokens_eq(
            "test-files/form/tokens/with_iter.rs",
            example_component().make_with_iter(),
        );
    }
    #[test]
    fn make_with_iter_mut() {
        assert_tokens_eq(
            "test-files/form/tokens/with_iter_mut.rs",
            example_component().make_with_iter_mut(),
        );
    }
    #[test]
    fn make_with_action_mut() {
        assert_tokens_eq(
            "test-files/form/tokens/with_action_mut.rs",
            example_component().make_with_action_mut(),
        );
    }
    #[test]
    fn append_show_if() {
        let mut out = TokenStream::new();
        example_component().append_show_if(&mut out);
        assert_tokens_eq("test-files/form/tokens/comp_show_if.rs", out);
    }
    #[test]
    fn make_index() {
        assert_tokens_eq(
            "test-files/form/tokens/comp_index.rs",
            example_component().make_index(),
        );
    }
    #[test]
    fn append_total_size() {
        let mut out = TokenStream::new();
        example_component().append_total_size(&mut out);
        assert_tokens_eq("test-files/form/tokens/comp_total_size.rs", out);
    }

    #[test]
    fn append_make_selection_default() {
        let mut out = TokenStream::new();
        example_component().append_make_selection_default(&mut out);
        assert_tokens_eq("test-files/form/tokens/comp_make_selection_default.rs", out);
    }
    #[test]
    fn make_impl_component_base() {
        let mut out = TokenStream::new();
        example_component().make_impl_component_base(&mut out);
        assert_tokens_eq("test-files/form/tokens/comp_base_full.rs", out);
    }
    #[test]
    fn make_impl_component() {
        let mut out = TokenStream::new();
        example_component().make_impl_component(&mut out);
        assert_tokens_eq("test-files/form/tokens/comp_full.rs", out);
    }
}
