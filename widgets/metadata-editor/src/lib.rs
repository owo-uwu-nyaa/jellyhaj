pub mod genre;

use std::convert::Infallible;

use jellyhaj_form_widget::{
    button::Button,
    form_component, form_widget,
    seperator::Seperator,
    text_field::{TextField, TextFieldDynamic},
};
use valuable::Valuable;

pub struct Mapper;

#[derive(Debug, Clone)]
pub enum MetadataActions {
    Update,
    AddGenre,
    RemoveGenre { id: String },
    AddTag,
}

impl From<Infallible> for MetadataActions {
    fn from(_value: Infallible) -> Self {
        unimplemented!()
    }
}

#[form_component(MetadataActions)]
#[derive(Debug, Valuable)]
pub struct ExternalId {
    #[form(descr = "")]
    id: TextFieldDynamic,
    #[form(skip)]
    key: String,
}

#[form_component(MetadataActions)]
#[derive(Debug, Valuable)]
pub struct ExternalIds {
    #[form(descr = "External Ids")]
    seperator: Seperator,
    #[form(flatten)]
    ids: Vec<ExternalId>,
}

#[form_component(MetadataActions)]
#[derive(Debug, Valuable)]
pub struct Genre {
    #[form(descr = "")]
    name: TextFieldDynamic,
    #[form(descr = "Remove genre")]
    button: Button<MetadataActions>,
}

#[form_widget("Edit Metadata", MetadataActions, Mapper)]
#[derive(Debug, Valuable)]
pub struct ModifyMetadata {
    #[form(descr = "Title")]
    title: TextField,
    #[form(descr = "Original title")]
    original_title: TextField,
    #[form(descr = "Sort title")]
    sort_title: TextField,
    #[form(descr = "Date added")]
    date_added: TextField,
    #[form(flatten, show_if(!self.external_id.ids.is_empty()))]
    external_id: ExternalIds,
    #[form(descr = "Genres")]
    gen_sep: Seperator,
    #[form(flatten)]
    genres: Vec<Genre>,
    #[form(descr = "Add genre")]
    add_genre: Button<MetadataActions>,
}
