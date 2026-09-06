fn with_iter<
    R: 'static,
    W: ::jellyhaj_form_widget::form::helpers::WithIterItems<R, Self::AR>,
>(
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
        += ::jellyhaj_form_widget::form::component::FormComponent::total_size(
            &self.flatten1,
        );
    ::jellyhaj_form_widget::form::component::FormComponent::with_iter(
        &self.flatten2,
        base_index,
        with,
    )?;
    base_index
        += ::jellyhaj_form_widget::form::component::FormComponent::total_size(
            &self.flatten2,
        );
    W::with(with, &self.simple3, "simple 3", base_index)?;
    base_index += 1;
    Ok(())
}
