use proc_macro2::{Delimiter, Group, Literal, TokenStream, TokenTree};
use quote::{TokenStreamExt, quote};
use syn::Token;

use crate::form::{Component, FieldKind};

impl Component {
    pub fn make_impl_component(&self, tokens: &mut TokenStream) {
        tokens.append_all(quote! {#[automatically_derived]});
        tokens.append_all([<Token![impl]>::default()]);
        tokens.append_all([&self.paths.form_component]);
        tokens.append_all([<Token![for]>::default()]);
        tokens.append_all([&self.data]);
        let mut impls = TokenStream::new();
        impls.append_all(self.make_component_type_defs());
        impls.append_all(self.make_with_selection());
        impls.append_all(self.make_with_selection_mut());
        impls.append_all(self.make_with_selection_mut_cx());
        self.append_with_index_mut(&mut impls);
        impls.append_all(self.make_with_iter());
        impls.append_all(self.make_with_iter_mut());
        impls.append_all(self.make_with_action_mut());
        self.append_show_if(&mut impls);
        impls.append_all(self.make_index());
        self.append_total_size(&mut impls);
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
        let component = &self.paths.form_component;
        let pats = self.fields.iter().enumerate().map(|(i, item)| {
            let name = &item.name;
            let ty = &item.ty;
            let sel = &item.selection;
            let index = self.make_base_index_expr(i);
            match &item.kind {
                FieldKind::Flatten => {
                    quote! {
                        #sel(s) => #component::with_selection(
                            &self.#name,
                            #index,
                            s,
                            with
                        )
                    }
                }
                FieldKind::Item { descr } => {
                    quote! {
                        #sel(s) => W::with::<#ty>(
                            with,
                            s,
                            &self.#name,
                            #descr,
                            #index
                        )
                    }
                }
            }
        });
        quote! {
            fn with_selection<T, W: #with_selection<Self::AR, T>>(
                &self,
                base_index: #exports::usize,
                this: &Self::Selector,
                with: W,
            ) -> T {
                match this {
                    #(#pats),*
                }
            }
        }
    }

    fn make_with_selection_mut(&self) -> TokenStream {
        let with_selection_mut = &self.paths.with_selection_mut;
        let exports = &self.paths.exports;
        let component = &self.paths.form_component;
        let pats = self.fields.iter().enumerate().map(|(i, item)| {
            let name = &item.name;
            let ty = &item.ty;
            let sel = &item.selection;
            let index = self.make_base_index_expr(i);
            match &item.kind {
                FieldKind::Flatten => {
                    quote! {
                        #sel(s) => #component::with_selection_mut(
                            &mut self.#name,
                            #index,
                            s,
                            with
                        )
                    }
                }
                FieldKind::Item { descr } => {
                    quote! {
                        #sel(s) => W::with_mut::<#ty>(
                            with,
                            s,
                            &mut self.#name,
                            #descr,
                            #index
                        )
                    }
                }
            }
        });
        quote! {
            fn with_selection_mut<T, W: #with_selection_mut<Self::AR, T>>(
                &mut self,
                base_index: #exports::usize,
                this: &mut Self::Selector,
                with: W,
            ) -> T {
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
            match &item.kind {
                FieldKind::Flatten => {
                    quote! {
                        #sel(s) => #component::with_selection_mut_cx(
                            &mut self.#name,
                            #index,
                            s,
                            cx.wrap_with(#action),
                            with
                        )
                    }
                }
                FieldKind::Item { descr } => {
                    quote! {
                        #sel(s) => W::with_mut::<#ty>(
                            with,
                            s,
                            cx.wrap_with(#action),
                            &mut self.#name,
                            #descr,
                            #index
                        )
                    }
                }
            }
        });
        quote! {
            fn with_selection_mut_cx<R: 'static, T, W: #with_selection_mut_cx<R, Self::AR, T>>(
                &mut self,
                base_index: #exports::usize,
                this: &mut Self::Selector,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                with: W,
            ) -> T {
                match this {
                    #(#pats),*
                }
            }
        }
    }

    fn append_with_index_mut(&self, stream: &mut TokenStream) {
        let with_index_mut = &self.paths.with_index_mut;
        let component = &self.paths.form_component;
        let exports = &self.paths.exports;

        stream.append_all(
        quote! {
            fn with_index_mut<R: 'static, W: #with_index_mut<R, Self::AR>>(
                &mut self,
                mut base_index: #exports::usize,
                this: &mut Self::Selector,
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
                match_body.append_all(quote! {
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
                body.append_all(quote! {
                    let cur = #component::total_size(&self.#name);
                    if index < base_index + cur {
                        let mut res = #exports::Default::default();
                        #component::with_index_mut(
                            &mut self.#name,
                            base_index
                            &mut res,
                            cx: cx.wrap_with(#action),
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
        let exports = &self.paths.exports;

        let items = self.fields.iter().map(|item| {
            let name = &item.name;
            match &item.kind {
                FieldKind::Item { descr } => {
                    quote! {
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
                    quote! {
                        #component::with_iter(
                            &self.name,
                            base_index,
                            with,
                        )?;
                        base_index += #component::total_size(&self.#name);
                    }
                }
            }
        });

        quote! {
            fn with_iter<R: 'static, W: #with_iter_items<R, Self::AR>>(
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
        let exports = &self.paths.exports;

        let items = self.fields.iter().map(|item| {
            let name = &item.name;
            let action = &item.action;
            let show = if let Some(fun) = item.show_if.as_ref().map(|v| &v.fun) {
                quote! {show && self.#fun()}
            } else {
                quote! {show}
            };
            match &item.kind {
                FieldKind::Item { descr } => {
                    quote! {
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
                FieldKind::Flatten => {
                    quote! {
                        let show = #show;
                        #component::with_iter_mut(
                            &mut self.name,
                            base_index,
                            cx.wrap_with(#action),
                            with,
                            show
                        )?;
                        base_index += #component::total_size(&self.#name);
                    }
                }
            }
        });

        quote! {
            fn with_iter_mut<R: 'static, W: #with_iter_items_mut<R, Self::AR>>(
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
            match &item.kind {
                FieldKind::Item { descr: _ } => {
                    quote! {
                        #action(a) => W::with_mut(
                            with,
                            a,
                            cx.wrap_with(#action),
                            &mut self.#name,
                            #index + base_index,
                        )
                    }
                }
                FieldKind::Flatten => {
                    quote! {
                        #action(a) => #component::with_action_mut(
                            &mut self.#name,
                            #index + base_index,
                            cx.wrap_with(#action),
                            with
                        )
                    }
                }
            }
        });

        quote! {
            fn with_action_mut<R: 'static, T, W: #with_action_mut<R, Self::AR, T>>(
                &mut self,
                base_index: #exports::usize,
                action: Self::Action,
                cx: #exports::WidgetContext<'_, Self::Action, impl #exports::Wrapper<Self::Action>, R>,
                with: W,
            ) -> T{
                match action {
                    #(#pats),*
                }
            }

        }
    }

    fn append_show_if(&self, stream: &mut TokenStream) {
        let component = &self.paths.form_component;
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
            if !{ match_body.is_empty() && always_show.is_empty() } {
                match_body.append_separated(
                    always_show.iter().copied().map(Literal::usize_suffixed),
                    <Token![|]>::default(),
                );
                match_body.append_all(quote! {=> return true,});
                match_body.append_all(quote! {_ => {}});

                body.append_all(quote! {match index});
                body.append(Group::new(Delimiter::Brace, match_body));
            }
            // flatten
            if let Some(item) = fields.next() {
                if processed > 0 {
                    let processed = Literal::usize_suffixed(processed);
                    body.append_all(quote! {index -= #processed});
                }
                let name = &item.name;
                let and = if let Some(fun) = item.show_if.as_ref().map(|s| &s.fun) {
                    quote! {self.#fun() &&}
                } else {
                    TokenStream::new()
                };
                body.append_all(quote! {
                    let cur = #component::total_size(&self.#name);
                    if index < cur {
                        return #and #component::show_if(&self.#name, index);
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
        let component = &self.paths.form_component;
        let exports = &self.paths.exports;
        let pats = self.fields.iter().enumerate().map(|(index, item)| {
            let index = self.make_base_index_expr(index);
            let sel = &item.selection;
            if item.is_item() {
                quote! {#sel(_) => #index,}
            } else {
                let name = &item.name;
                quote! {
                    #sel(sel) => #index + #component::index(&self.#name, sel),
                }
            }
        });
        quote! {
            fn index(&self, sel: &Self::Selector) -> #exports::usize{
                match sel {
                    #(#pats)*
                }
            }
        }
    }

    fn append_total_size(&self, stream: &mut TokenStream) {
        let exports = &self.paths.exports;
        stream.append_all(quote! {fn total_size(&self) -> #exports::usize});
        stream.append(self.make_base_index_expr(self.fields.len()));
    }

    fn make_base_index_expr(&self, index: usize) -> Group {
        let mut offset = 0usize;
        let component = &self.paths.form_component;
        let folded = self.fields[0..index].iter().filter_map(|item| {
            if matches!(item.kind, FieldKind::Flatten) {
                let name = &item.name;
                Some(quote! {#component::total_size(&self.#name)})
            } else {
                offset += 1;
                None
            }
        });
        let mut res = TokenStream::new();
        res.append_terminated(folded, <Token![+]>::default());
        res.append(Literal::usize_suffixed(offset));
        Group::new(Delimiter::Brace, res)
    }
}
