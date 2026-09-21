#[derive(::jellyhaj_form_widget::macro_impl::exports::Debug)]
pub enum ExampleAction {
    Simple1(
        <bool as ::jellyhaj_form_widget::FormItemBase<
            crate::ExampleActionResult,
        >>::Action,
    ),
    Simple2(
        <&'static str as ::jellyhaj_form_widget::FormItemBase<
            crate::ExampleActionResult,
        >>::Action,
    ),
    Flatten1(
        <Comp1 as ::jellyhaj_form_widget::form::component::FormComponentBase>::Action,
    ),
    Flatten2(
        <crate::Comp2 as ::jellyhaj_form_widget::form::component::FormComponentBase>::Action,
    ),
    Simple3(
        <Simple as ::jellyhaj_form_widget::FormItemBase<
            crate::ExampleActionResult,
        >>::Action,
    ),
}
