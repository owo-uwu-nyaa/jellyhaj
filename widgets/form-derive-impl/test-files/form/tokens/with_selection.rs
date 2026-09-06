fn with_selection<W: ::jellyhaj_form_widget::form::helpers::WithSelection<Self::AR>>(
    &self,
    base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    this: &Self::Selector,
    with: W,
) -> ::jellyhaj_form_widget::macro_impl::exports::bool {
    match this {
        ExampleSelection::Simple1(s) => {
            W::with::<bool>(with, s, &self.simple1, "simple 1", (base_index + 0usize))
        }
        ExampleSelection::Simple2(s) => {
            W::with::<
                &'static str,
            >(with, s, &self.simple2, "simple 2", (base_index + 1usize))
        }
        ExampleSelection::Flatten1(s) => {
            ::jellyhaj_form_widget::form::component::FormComponent::with_selection(
                &self.flatten1,
                (base_index + 2usize),
                s,
                with,
            )
        }
        ExampleSelection::Flatten2(s) => {
            ::jellyhaj_form_widget::form::component::FormComponent::with_selection(
                &self.flatten2,
                (base_index
                    + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
                        &self.flatten1,
                    ) + 2usize),
                s,
                with,
            )
        }
        ExampleSelection::Simple3(s) => {
            W::with::<
                Simple,
            >(
                with,
                s,
                &self.simple3,
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
