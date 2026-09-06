fn with_iter_mut<
    R: 'static,
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
            += ::jellyhaj_form_widget::form::component::FormComponent::total_size(
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
            += ::jellyhaj_form_widget::form::component::FormComponent::total_size(
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
