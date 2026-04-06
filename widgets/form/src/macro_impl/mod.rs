pub mod exports {
    pub use color_eyre::Result;
    pub use jellyhaj_widgets_core::{
        WidgetContext, Wrapper,
        valuable::{EnumDef, Enumerable, Fields, Valuable, Value, Variant, VariantDef, Visit},
    };
    pub use std::{
        panic,
        primitive::{bool, str, usize},
    };
}
