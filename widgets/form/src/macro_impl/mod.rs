pub mod exports {
    pub use color_eyre::Result;
    pub use jellyhaj_widgets_core::{
        WidgetContext, Wrapper,
        valuable::{EnumDef, Enumerable, Fields, Valuable, Value, Variant, VariantDef, Visit},
    };
    pub use std::{
        default::Default,
        fmt::Debug,
        option::Option,
        panic,
        primitive::{bool, str, usize},
    };

    use crate::{FormItemBase, form::component::FormComponent};
    use std::{convert::Infallible, marker::PhantomData};

    pub struct TypeCheck<AR: Debug + From<Infallible>> {
        _ar: PhantomData<AR>,
    }
    impl<AR: Debug + From<Infallible>> TypeCheck<AR> {
        pub const fn is_form_component<F: FormComponent<AR = AR>>() {}
        pub const fn is_form_item<I: FormItemBase<AR>>() {}
    }
}
