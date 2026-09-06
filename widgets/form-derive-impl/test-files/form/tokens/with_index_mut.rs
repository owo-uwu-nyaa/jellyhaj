fn with_index_mut<
    R: 'static,
    W: ::jellyhaj_form_widget::form::helpers::WithIndexMut<R, Self::AR>,
>(
    &mut self,
    mut base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    this: &mut Self::Selector,
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
    let cur = ::jellyhaj_form_widget::form::component::FormComponent::total_size(
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
    let cur = ::jellyhaj_form_widget::form::component::FormComponent::total_size(
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
