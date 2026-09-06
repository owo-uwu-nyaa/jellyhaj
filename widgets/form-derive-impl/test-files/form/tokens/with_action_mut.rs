fn with_action_mut<
    R: 'static,
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
                    + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
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
                    + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
                        &self.flatten1,
                    )
                    + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
                        &self.flatten2,
                    ) + 2usize),
            )
        }
    }
}
