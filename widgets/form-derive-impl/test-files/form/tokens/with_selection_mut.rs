fn with_selection_mut<
    W: ::jellyhaj_form_widget::form::helpers::WithSelectionMut<Self::AR>,
>(
    &mut self,
    base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    this: &mut Self::Selector,
    with: W,
) {
    match this {
        ExampleSelection::Simple1(s) => {
            W::with_mut::<
                bool,
            >(with, s, &mut self.simple1, "simple 1", (base_index + 0usize))
        }
        ExampleSelection::Simple2(s) => {
            W::with_mut::<
                &'static str,
            >(with, s, &mut self.simple2, "simple 2", (base_index + 1usize))
        }
        ExampleSelection::Flatten1(s) => {
            ::jellyhaj_form_widget::form::component::FormComponent::with_selection_mut(
                &mut self.flatten1,
                (base_index + 2usize),
                s,
                with,
            )
        }
        ExampleSelection::Flatten2(s) => {
            ::jellyhaj_form_widget::form::component::FormComponent::with_selection_mut(
                &mut self.flatten2,
                (base_index
                    + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
                        &self.flatten1,
                    ) + 2usize),
                s,
                with,
            )
        }
        ExampleSelection::Simple3(s) => {
            W::with_mut::<
                Simple,
            >(
                with,
                s,
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
