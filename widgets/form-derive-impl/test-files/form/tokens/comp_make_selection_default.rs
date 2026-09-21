fn make_selection_default(
    &self,
    mut base_index: ::jellyhaj_form_widget::macro_impl::exports::usize,
    index: ::jellyhaj_form_widget::macro_impl::exports::usize,
) -> ExampleSelection {
    match index - base_index {
        0usize => {
            return ExampleSelection::Simple1(
                ::jellyhaj_form_widget::macro_impl::exports::Default::default(),
            );
        }
        1usize => {
            return ExampleSelection::Simple2(
                ::jellyhaj_form_widget::macro_impl::exports::Default::default(),
            );
        }
        _ => {}
    }
    base_index += 2usize;
    let cur = ::jellyhaj_form_widget::form::component::FormComponent::total_size(
        &self.flatten1,
    );
    if index < base_index + cur {
        return ExampleSelection::Flatten1(
            &self.flatten1,
            ::jellyhaj_form_widget::form::component::FormComponentBase::make_selection_default(
                base_index,
                index,
            ),
        )
    } else {
        base_index += cur;
    }
    let cur = ::jellyhaj_form_widget::form::component::FormComponent::total_size(
        &self.flatten2,
    );
    if index < base_index + cur {
        return ExampleSelection::Flatten2(
            &self.flatten2,
            ::jellyhaj_form_widget::form::component::FormComponentBase::make_selection_default(
                base_index,
                index,
            ),
        )
    } else {
        base_index += cur;
    }
    match index - base_index {
        0usize => {
            return ExampleSelection::Simple3(
                ::jellyhaj_form_widget::macro_impl::exports::Default::default(),
            );
        }
        _ => {}
    }
    ::jellyhaj_form_widget::macro_impl::exports::panic!("index out of bounds")
}
