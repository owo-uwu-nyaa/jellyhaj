fn with_selection_mut_cx<
    R: 'static,
    T: ::jellyhaj_form_widget::macro_impl::exports::Default,
    W: ::jellyhaj_form_widget::form::helpers::WithSelectionMutCX<R, Self::AR, T>,
>(
    &mut self,
    base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    this: &mut Self::Selector,
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
                    + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
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
