pub mod genre;

use std::{
    convert::Infallible,
    fmt::Debug,
    str::FromStr,
    sync::{Arc, LazyLock},
};

use jellyfin::items::{Culture, MediaItem, MetadataEditor, MetadataUpdate};
use jellyhaj_core::{
    keybinds::FormCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_form_widget::{
    FormAction, Selection,
    button::{Button, SymbolButton},
    form::{
        Form, FormCommandMapper, FormResultMapper,
        component::{ComponentVec, FormComponentBase},
    },
    form_component, form_widget,
    label::DynamicLabel,
    selection::{DynamicSelection, DynamicSelectionItem},
    seperator::Seperator,
    text_block::TextBlock,
    text_field::{TextField, TextFieldDynamic},
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    Config, ContextRef, Result, WidgetContext, Wrapper,
    async_task::ErasedSubmitter,
    mapper::{ActionMapper, ActionMapperBase},
    outer::UnwrapWidget,
};
use jiff::{civil::DateTime, tz::TimeZone};
use valuable::Valuable;

pub struct Mapper;

fn clone_opt(v: &str) -> Option<String> {
    if v.is_empty() {
        None
    } else {
        Some(v.to_owned())
    }
}

impl FormResultMapper<ModifyMetadata> for Mapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<ModifyMetadata>,
        form_result: <ModifyMetadata as FormComponentBase>::AR,
        _cx: WidgetContext<
            '_,
            FormAction<<ModifyMetadata as FormComponentBase>::Action>,
            impl Wrapper<FormAction<<ModifyMetadata as FormComponentBase>::Action>>,
            (),
        >,
        render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<Self::Res>> {
        match form_result {
            MetadataActions::Update => {
                let date_created = if state.data.date_added.text.is_empty() {
                    None
                } else {
                    let Ok(time) = DateTime::from_str(&state.data.date_added.text) else {
                        return Ok(None);
                    };
                    let Ok(time) = time.to_zoned(LOCAL_ZONE.clone()) else {
                        return Ok(None);
                    };
                    Some(time.timestamp())
                };
                let status = match state.data.status {
                    Status::Continuing => "Continuing",
                    Status::Ended => "Ended",
                    Status::Unreleased => "Unreleased",
                    Status::Unknown => "",
                };
                Ok(Some(Navigation::Replace(NextScreen::DoModifyMetadata {
                    id: state.data.media_item.id.clone(),
                    new_metadata: Box::new(MetadataUpdate {
                        name: state.data.title.text.clone(),
                        original_title: state.data.original_title.text.clone(),
                        sort_name: state.data.sort_title.text.clone(),
                        original_language: state
                            .data
                            .original_language
                            .get()
                            .inner
                            .as_ref()
                            .map_or_else(Default::default, |v| {
                                v.three_letter_iso_language_name.clone()
                            }),
                        date_created,
                        status,
                        overview: clone_opt(&state.data.overview.text),
                        genres: state
                            .data
                            .genres
                            .iter()
                            .map(|g| g.button.inner.val.clone())
                            .collect(),
                    }),
                })))
            }
            MetadataActions::AddGenre => Ok(Some(Navigation::Push(NextScreen::AddGenreFetch {
                result_sender: state
                    .data
                    .new_genre_submit
                    .as_ref()
                    .expect("new genre submitter not available, was action mapper init called?")
                    .clone(),
                selected: state
                    .data
                    .genres
                    .iter()
                    .map(|genre| genre.button.inner.val.clone())
                    .collect(),
            }))),
            MetadataActions::RemoveGenre { id } => {
                state
                    .data
                    .genres
                    .retain(|genre| genre.button.inner.val != id);
                render_flag.set();
                Ok(None)
            }
            MetadataActions::AddTag => todo!(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MetadataActions {
    Update,
    AddGenre,
    RemoveGenre { id: String },
    AddTag,
}

impl From<Infallible> for MetadataActions {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[form_component(MetadataActions)]
#[derive(Debug, Valuable)]
pub struct ExternalId {
    #[form(descr = "")]
    id: TextFieldDynamic,
}

#[form_component(MetadataActions)]
#[derive(Debug, Valuable)]
pub struct ExternalIds {
    #[form(descr = "External Ids")]
    seperator: Seperator,
    #[form(flatten)]
    ids: ComponentVec<ExternalId>,
}

#[form_component(MetadataActions)]
#[derive(Debug, Valuable)]
pub struct Genre {
    #[form(descr = "")]
    button: SymbolButton<MetadataActions, DynamicLabel>,
}

struct CultureItem {
    inner: Option<Culture>,
}

impl Valuable for CultureItem {
    fn as_value(&self) -> valuable::Value<'_> {
        self.inner.as_value()
    }

    fn visit(&self, visit: &mut dyn valuable::Visit) {
        self.inner.visit(visit);
    }
}

impl Debug for CultureItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.inner, f)
    }
}

impl DynamicSelectionItem for CultureItem {
    fn name(&self) -> &str {
        self.inner.as_ref().map_or("", |v| &v.display_name)
    }
}

#[derive(Debug, Selection, Valuable, PartialEq, Eq, Clone, Copy)]
enum Status {
    #[descr("Continuing")]
    Continuing,
    #[descr("Ended")]
    Ended,
    #[descr("Not yet released")]
    Unreleased,
    #[descr("Unknown")]
    Unknown,
}

#[form_widget("Edit Metadata", MetadataActions, Mapper, ContextRef<Config>)]
#[derive(Debug, Valuable)]
pub struct ModifyMetadata {
    #[form(descr = "Title")]
    title: TextField,
    #[form(descr = "Original title")]
    original_title: TextField,
    #[form(descr = "Original language")]
    original_language: DynamicSelection<CultureItem>,
    #[form(descr = "Sort title")]
    sort_title: TextField,
    #[form(descr = "Date added")]
    date_added: TextField,
    #[form(descr = "Status")]
    status: Status,
    #[form(descr = "Overview")]
    overview: TextBlock,
    #[form(flatten, show_if(!self.external_id.ids.is_empty()))]
    external_id: ExternalIds,
    #[form(descr = "Genres")]
    gen_sep: Seperator,
    #[form(flatten)]
    genres: ComponentVec<Genre>,
    #[form(descr = "Add genre")]
    add_genre: Button<MetadataActions>,
    #[form(descr = "Update item")]
    update: Button<MetadataActions>,
    #[form(skip)]
    #[valuable(skip)]
    new_genre_submit: Option<Arc<dyn ErasedSubmitter<String>>>,
    #[form(skip)]
    media_item: Box<MediaItem>,
}

static LOCAL_ZONE: LazyLock<TimeZone> = LazyLock::new(TimeZone::system);

impl ModifyMetadata {
    #[must_use]
    pub fn new(item: Box<MediaItem>, editor: MetadataEditor) -> Self {
        let mut original_language = Vec::with_capacity(editor.cultures.len() + 1);
        original_language.push(CultureItem { inner: None });
        original_language.extend(
            editor
                .cultures
                .into_iter()
                .map(|c| CultureItem { inner: Some(c) }),
        );
        let mut original_language = DynamicSelection::new(original_language);
        if let Some(lang) = &item.original_language
            && !lang.is_empty()
        {
            original_language.select(|c| {
                if let Some(c) = &c.inner {
                    &c.two_letter_iso_language_name == lang
                        || &c.three_letter_iso_language_name == lang
                        || c.three_letter_iso_language_names.iter().any(|c| c == lang)
                } else {
                    false
                }
            });
        }
        Self {
            title: TextField::new(item.name.clone()),
            original_title: TextField::new(item.original_title.clone().unwrap_or_default()),

            sort_title: TextField::new(item.sort_name.clone().unwrap_or_default()),
            date_added: TextField::with_checker(
                item.date_created
                    .as_ref()
                    .map(|d| d.to_zoned(LOCAL_ZONE.clone()).strftime("%F %T").to_string())
                    .unwrap_or_default(),
                |v| DateTime::from_str(v).is_ok(),
            ),
            original_language,
            status: match item.status.as_deref() {
                Some("Continuing") => Status::Continuing,
                Some("Ended") => Status::Ended,
                Some("Unreleased") => Status::Unreleased,
                _ => Status::Unknown,
            },
            external_id: ExternalIds {
                seperator: Seperator,
                ids: item
                    .provider_ids
                    .iter()
                    .map(|(provider, id)| ExternalId {
                        id: TextFieldDynamic::new(id.clone(), provider.clone()),
                    })
                    .collect(),
            },
            gen_sep: Seperator,
            genres: item
                .genre_items
                .iter()
                .map(|genre| Genre {
                    button: SymbolButton::new(
                        MetadataActions::RemoveGenre {
                            id: genre.name.clone(),
                        },
                        '-',
                        DynamicLabel::new(genre.name.clone()),
                    ),
                })
                .collect(),
            add_genre: Button::new(MetadataActions::AddGenre),
            new_genre_submit: None,
            update: Button::new(MetadataActions::Update),
            overview: TextBlock::new(item.overview.clone().unwrap_or_default()),
            media_item: item,
        }
    }
}

#[derive(Valuable)]
pub struct ModifyMetadataActionMapper;

type InnerWidget = UnwrapWidget<
    KeybindWidget<
        FormCommand,
        UnwrapWidget<ModifyMetadataWidget>,
        FormCommandMapper<ModifyMetadataAction>,
    >,
>;

#[derive(Debug)]
pub enum SubformResult {
    AddGenre(String),
}

impl ActionMapperBase<InnerWidget> for ModifyMetadataActionMapper {
    type Action = SubformResult;
}

impl<R: ContextRef<Config> + 'static> ActionMapper<R, InnerWidget> for ModifyMetadataActionMapper {
    fn init(
        &mut self,
        this: &mut InnerWidget,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: WidgetContext<
            '_,
            <InnerWidget as jellyhaj_widgets_core::JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as jellyhaj_widgets_core::JellyhajWidgetBase>::Action>,
            R,
        >,
    ) {
        this.inner.inner.inner.data.new_genre_submit = Some(Arc::new(
            cx.submitter.wrap_with(SubformResult::AddGenre).erased(),
        ));
    }

    fn map_action(
        &mut self,
        this: &mut InnerWidget,
        _cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: WidgetContext<
            '_,
            <InnerWidget as jellyhaj_widgets_core::JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as jellyhaj_widgets_core::JellyhajWidgetBase>::Action>,
            R,
        >,
        action: Self::Action,
        render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<<InnerWidget as jellyhaj_widgets_core::JellyhajWidgetBase>::ActionResult>>
    {
        match action {
            SubformResult::AddGenre(new_genre) => {
                let button = SymbolButton::new(
                    MetadataActions::RemoveGenre {
                        id: new_genre.clone(),
                    },
                    '-',
                    DynamicLabel::new(new_genre),
                );
                this.inner.inner.inner.data.genres.push(Genre { button });
                render_flag.set();
                Ok(None)
            }
        }
    }
}
