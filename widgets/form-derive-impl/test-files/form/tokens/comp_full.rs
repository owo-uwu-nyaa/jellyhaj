#[allow(unused_parens)]
#[automatically_derived]
impl<R: 'static> ::jellyhaj_form_widget::form::component::FormComponent<R> for Example {
    fn with_selection_mut_cx<
        T: ::jellyhaj_form_widget::macro_impl::exports::Default,
        W: ::jellyhaj_form_widget::form::helpers::WithSelectionMutCX<R, Self::AR, T>,
    >(
        &mut self,
        base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        this: &mut ExampleSelection,
        cx: ::jellyhaj_form_widget::macro_impl::exports::WidgetContext<
            '_,
            Self::Action,
            impl ::jellyhaj_form_widget::macro_impl::exports::Wrapper<Self::Action>,
            R,
        >,
        with: W,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::Result<T> {
        match this {
            ExampleSelection::Simple1(s) => {
                W::with_mut::<
                    bool,
                >(
                    with,
                    s,
                    cx.wrap_with(ExampleAction::Simple1),
                    &mut self.simple1,
                    "simple 1",
                    (base_index + 0usize),
                )
            }
            ExampleSelection::Simple2(s) => {
                W::with_mut::<
                    &'static str,
                >(
                    with,
                    s,
                    cx.wrap_with(ExampleAction::Simple2),
                    &mut self.simple2,
                    "simple 2",
                    (base_index + 1usize),
                )
            }
            ExampleSelection::Flatten1(s) => {
                ::jellyhaj_form_widget::form::component::FormComponent::with_selection_mut_cx(
                    &mut self.flatten1,
                    (base_index + 2usize),
                    s,
                    cx.wrap_with(ExampleAction::Flatten1),
                    with,
                )
            }
            ExampleSelection::Flatten2(s) => {
                ::jellyhaj_form_widget::form::component::FormComponent::with_selection_mut_cx(
                    &mut self.flatten2,
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        ) + 2usize),
                    s,
                    cx.wrap_with(ExampleAction::Flatten2),
                    with,
                )
            }
            ExampleSelection::Simple3(s) => {
                W::with_mut::<
                    Simple,
                >(
                    with,
                    s,
                    cx.wrap_with(ExampleAction::Simple3),
                    &mut self.simple3,
                    "simple 3",
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        )
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten2,
                        ) + 2usize),
                )
            }
        }
    }
    fn with_index_mut<
        W: ::jellyhaj_form_widget::form::helpers::WithIndexMut<R, Self::AR>,
    >(
        &mut self,
        mut base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        this: &mut ExampleSelection,
        cx: ::jellyhaj_form_widget::macro_impl::exports::WidgetContext<
            '_,
            Self::Action,
            impl ::jellyhaj_form_widget::macro_impl::exports::Wrapper<Self::Action>,
            R,
        >,
        index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        with: W,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::Result<()> {
        match index - base_index {
            0usize => {
                *this = ExampleSelection::Simple1(
                    W::with_mut(
                        with,
                        cx.wrap_with(ExampleAction::Simple1),
                        &mut self.simple1,
                        "simple 1",
                        base_index + 0usize,
                    )?,
                );
                return Ok(());
            }
            1usize => {
                *this = ExampleSelection::Simple2(
                    W::with_mut(
                        with,
                        cx.wrap_with(ExampleAction::Simple2),
                        &mut self.simple2,
                        "simple 2",
                        base_index + 1usize,
                    )?,
                );
                return Ok(());
            }
            _ => {}
        }
        base_index += 2usize;
        let cur = ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten1,
        );
        if index < base_index + cur {
            let mut res = ::jellyhaj_form_widget::macro_impl::exports::Default::default();
            ::jellyhaj_form_widget::form::component::FormComponent::with_index_mut(
                &mut self.flatten1,
                base_index,
                &mut res,
                cx.wrap_with(ExampleAction::Flatten1),
                index,
                with,
            )?;
            *this = ExampleSelection::Flatten1(res);
            return Ok(());
        } else {
            base_index += cur;
        }
        let cur = ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
            &self.flatten2,
        );
        if index < base_index + cur {
            let mut res = ::jellyhaj_form_widget::macro_impl::exports::Default::default();
            ::jellyhaj_form_widget::form::component::FormComponent::with_index_mut(
                &mut self.flatten2,
                base_index,
                &mut res,
                cx.wrap_with(ExampleAction::Flatten2),
                index,
                with,
            )?;
            *this = ExampleSelection::Flatten2(res);
            return Ok(());
        } else {
            base_index += cur;
        }
        match index - base_index {
            0usize => {
                *this = ExampleSelection::Simple3(
                    W::with_mut(
                        with,
                        cx.wrap_with(ExampleAction::Simple3),
                        &mut self.simple3,
                        "simple 3",
                        base_index + 0usize,
                    )?,
                );
                return Ok(());
            }
            _ => {}
        }
        ::jellyhaj_form_widget::macro_impl::exports::panic!("index out of bounds")
    }
    fn with_iter<W: ::jellyhaj_form_widget::form::helpers::WithIterItems<R, Self::AR>>(
        &self,
        mut base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        with: &mut W,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::Result<()> {
        W::with(with, &self.simple1, "simple 1", base_index)?;
        base_index += 1;
        W::with(with, &self.simple2, "simple 2", base_index)?;
        base_index += 1;
        ::jellyhaj_form_widget::form::component::FormComponent::with_iter(
            &self.flatten1,
            base_index,
            with,
        )?;
        base_index
            += ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                &self.flatten1,
            );
        ::jellyhaj_form_widget::form::component::FormComponent::with_iter(
            &self.flatten2,
            base_index,
            with,
        )?;
        base_index
            += ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                &self.flatten2,
            );
        W::with(with, &self.simple3, "simple 3", base_index)?;
        base_index += 1;
        Ok(())
    }
    fn with_iter_mut<
        W: ::jellyhaj_form_widget::form::helpers::WithIterItemsMut<R, Self::AR>,
    >(
        &mut self,
        mut base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        cx: ::jellyhaj_form_widget::macro_impl::exports::WidgetContext<
            '_,
            Self::Action,
            impl ::jellyhaj_form_widget::macro_impl::exports::Wrapper<Self::Action>,
            R,
        >,
        with: &mut W,
        show: bool,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::Result<()> {
        {
            let show = show;
            W::with_mut(
                with,
                cx.wrap_with(ExampleAction::Simple1),
                &mut self.simple1,
                "simple 1",
                base_index,
                show,
            )?;
            base_index += 1;
        }
        {
            let show = show && self._show_if_simple2();
            W::with_mut(
                with,
                cx.wrap_with(ExampleAction::Simple2),
                &mut self.simple2,
                "simple 2",
                base_index,
                show,
            )?;
            base_index += 1;
        }
        {
            let show = show;
            ::jellyhaj_form_widget::form::component::FormComponent::with_iter_mut(
                &mut self.flatten1,
                base_index,
                cx.wrap_with(ExampleAction::Flatten1),
                with,
                show,
            )?;
            base_index
                += ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                    &self.flatten1,
                );
        }
        {
            let show = show && self._show_if_flatten2();
            ::jellyhaj_form_widget::form::component::FormComponent::with_iter_mut(
                &mut self.flatten2,
                base_index,
                cx.wrap_with(ExampleAction::Flatten2),
                with,
                show,
            )?;
            base_index
                += ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                    &self.flatten2,
                );
        }
        {
            let show = show;
            W::with_mut(
                with,
                cx.wrap_with(ExampleAction::Simple3),
                &mut self.simple3,
                "simple 3",
                base_index,
                show,
            )?;
            base_index += 1;
        }
        Ok(())
    }
    fn with_action_mut<
        T,
        W: ::jellyhaj_form_widget::form::helpers::WithActionMut<R, Self::AR, T>,
    >(
        &mut self,
        base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
        action: Self::Action,
        cx: ::jellyhaj_form_widget::macro_impl::exports::WidgetContext<
            '_,
            Self::Action,
            impl ::jellyhaj_form_widget::macro_impl::exports::Wrapper<Self::Action>,
            R,
        >,
        with: W,
    ) -> ::jellyhaj_form_widget::macro_impl::exports::Result<
        ::jellyhaj_form_widget::macro_impl::exports::Option<T>,
    > {
        match action {
            ExampleAction::Simple1(a) => {
                W::with_mut(
                    with,
                    a,
                    cx.wrap_with(ExampleAction::Simple1),
                    &mut self.simple1,
                    (base_index + 0usize),
                )
            }
            ExampleAction::Simple2(a) => {
                W::with_mut(
                    with,
                    a,
                    cx.wrap_with(ExampleAction::Simple2),
                    &mut self.simple2,
                    (base_index + 1usize),
                )
            }
            ExampleAction::Flatten1(a) => {
                ::jellyhaj_form_widget::form::component::FormComponent::with_action_mut(
                    &mut self.flatten1,
                    (base_index + 2usize),
                    a,
                    cx.wrap_with(ExampleAction::Flatten1),
                    with,
                )
            }
            ExampleAction::Flatten2(a) => {
                ::jellyhaj_form_widget::form::component::FormComponent::with_action_mut(
                    &mut self.flatten2,
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        ) + 2usize),
                    a,
                    cx.wrap_with(ExampleAction::Flatten2),
                    with,
                )
            }
            ExampleAction::Simple3(a) => {
                W::with_mut(
                    with,
                    a,
                    cx.wrap_with(ExampleAction::Simple3),
                    &mut self.simple3,
                    (base_index
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten1,
                        )
                        + ::jellyhaj_form_widget::form::component::FormComponentBase::total_size(
                            &self.flatten2,
                        ) + 2usize),
                )
            }
        }
    }
}
