#[derive(::jellyhaj_form_widget::macro_impl::exports::Debug)]
pub enum ExampleSelection {
    Simple1(
        <bool as ::jellyhaj_form_widget::FormItemBase<
            crate::ExampleActionResult,
        >>::SelectionInner,
    ),
    Simple2(
        <&'static str as ::jellyhaj_form_widget::FormItemBase<
            crate::ExampleActionResult,
        >>::SelectionInner,
    ),
    Flatten1(
        <Comp1 as ::jellyhaj_form_widget::form::component::FormComponent>::Selector,
    ),
    Flatten2(
        <crate::Comp2 as ::jellyhaj_form_widget::form::component::FormComponent>::Selector,
    ),
    Simple3(
        <Simple as ::jellyhaj_form_widget::FormItemBase<
            crate::ExampleActionResult,
        >>::SelectionInner,
    ),
}
