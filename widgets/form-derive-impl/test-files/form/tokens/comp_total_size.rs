fn total_size(&self) -> ::jellyhaj_form_widget::macro_impl::exports::usize {
    ::jellyhaj_form_widget::form::component::FormComponent::total_size(&self.flatten1)
        + ::jellyhaj_form_widget::form::component::FormComponent::total_size(
            &self.flatten2,
        ) + 3usize
}
