fn show_if(
    &self,
    mut index: ::jellyhaj_form_widget::macro_impl::exports::usize,
) -> ::jellyhaj_form_widget::macro_impl::exports::bool {
    match index {
        1usize => return self._show_if_simple2(),
        0usize => return true,
        _ => {}
    }
    index -= 2usize;
    let cur = ::jellyhaj_form_widget::form::component::FormComponent::total_size(
        &self.flatten1,
    );
    if index < cur {
        return ::jellyhaj_form_widget::form::component::FormComponent::show_if(
            &self.flatten1,
            index,
        );
    }
    index -= cur;
    let cur = ::jellyhaj_form_widget::form::component::FormComponent::total_size(
        &self.flatten2,
    );
    if index < cur {
        return self._show_if_flatten2()
            && ::jellyhaj_form_widget::form::component::FormComponent::show_if(
                &self.flatten2,
                index,
            );
    }
    index -= cur;
    match index {
        0usize => return true,
        _ => {}
    };
    ::jellyhaj_form_widget::macro_impl::exports::panic!("index out of bounds")
}
