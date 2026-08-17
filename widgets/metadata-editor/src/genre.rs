use std::convert::Infallible;

use jellyhaj_form_widget::{
    button::Button, form_widget, selection::DynamicSelection, text_field::TextField,
};
use valuable::Valuable;

#[derive(Debug, Clone, Copy)]
pub enum GenreDo {
    Add,
    Select,
}

pub struct GenreSelection {
    pub existing: DynamicSelection,
    pub add: Button<GenreDo>,
    pub select: Button<GenreDo>,
}

pub struct AddMapper;

#[derive(Debug, Clone, Copy)]
pub struct Add;

impl From<Infallible> for Add {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}

#[form_widget("Add new genre", Add, AddMapper)]
#[derive(Debug, Valuable)]
pub struct AddGenre {
    #[form(descr = "Genre name")]
    pub name: TextField,
    #[form(descr = "Add")]
    pub add: Button<Add>,
}
