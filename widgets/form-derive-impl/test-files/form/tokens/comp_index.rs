fn index(
    &self,
    sel: &Self::Selector,
) -> ::jellyhaj_form_widget::macro_impl::exports::usize {
    match sel {
        ExampleSelection::Simple1(_) => 0usize,
        ExampleSelection::Simple2(_) => 1usize,
        ExampleSelection::Flatten1(sel) => {
            (2usize
                + ::jellyhaj_form_widget::form::component::FormComponent::index(
                    &self.flatten1,
                    sel,
                ))
        }
        ExampleSelection::Flatten2(sel) => {
            (::jellyhaj_form_widget::form::component::FormComponent::total_size(
                &self.flatten1,
            ) + 2usize
                + ::jellyhaj_form_widget::form::component::FormComponent::index(
                    &self.flatten2,
                    sel,
                ))
        }
        ExampleSelection::Simple3(_) => {
            ::jellyhaj_form_widget::form::component::FormComponent::total_size(
                &self.flatten1,
            )
                + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
                    &self.flatten2,
                ) + 2usize
        }
    }
}
