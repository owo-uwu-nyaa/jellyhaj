pub mod genre;

use std::{convert::Infallible, sync::Arc};

use jellyfin::items::{MediaItem, MetadataEditor, MetadataUpdate};
use jellyhaj_core::{
    Config,
    keybinds::FormCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_form_widget::{
    FormAction,
    button::{Button, DynamicButton},
    form::{
        Form, FormCommandMapper, FormResultMapper,
        component::{ComponentVec, FormComponent},
    },
    form_component, form_widget,
    seperator::Seperator,
    text_field::{TextField, TextFieldDynamic},
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, Result, WidgetContext, Wrapper,
    async_task::ErasedSubmitter,
    mapper::{ActionMapper, ActionMapperBase},
    outer::UnwrapWidget,
};
use valuable::Valuable;

pub struct Mapper;

impl FormResultMapper<ModifyMetadata> for Mapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<ModifyMetadata>,
        form_result: <ModifyMetadata as FormComponent>::AR,
        _cx: WidgetContext<
            '_,
            FormAction<<ModifyMetadata as FormComponent>::Action>,
            impl Wrapper<FormAction<<ModifyMetadata as FormComponent>::Action>>,
            (),
        >,
        render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<Self::Res>> {
        match form_result {
            MetadataActions::Update => {
                Ok(Some(Navigation::Replace(NextScreen::DoModifyMetadata {
                    id: state.data.media_item.id.clone(),
                    new_metadata: Box::new(MetadataUpdate {
                        name: state.data.title.text.clone(),
                        original_title: state.data.original_title.text.clone(),
                        sort_name: state.data.sort_title.text.clone(),
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
                    .map(|genre| genre.button.name.clone())
                    .collect(),
            }))),
            MetadataActions::RemoveGenre { id } => {
                state.data.genres.retain(|genre| genre.button.name != id);
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
    fn from(_value: Infallible) -> Self {
        unimplemented!()
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
    #[form(descr = "Remove genre")]
    button: DynamicButton<MetadataActions>,
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
    genres: ComponentVec<Genre>,
    #[form(descr = "Add genre")]
    add_genre: Button<MetadataActions>,
    #[form(skip)]
    #[valuable(skip)]
    new_genre_submit: Option<Arc<dyn ErasedSubmitter<String>>>,
    #[form(skip)]
    media_item: Box<MediaItem>,
}

impl ModifyMetadata {
    #[must_use]
    pub fn new(item: Box<MediaItem>, _editor: MetadataEditor) -> Self {
        Self {
            title: TextField::new(item.name.clone()),
            original_title: TextField::new(item.original_title.clone().unwrap_or_default()),
            sort_title: TextField::new(item.sort_name.clone().unwrap_or_default()),
            date_added: TextField::new(item.date_created.clone().unwrap_or_default()),
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
                    button: DynamicButton::new(
                        genre.name.clone(),
                        MetadataActions::RemoveGenre {
                            id: genre.name.clone(),
                        },
                    ),
                })
                .collect(),
            add_genre: Button::new(MetadataActions::AddGenre),
            new_genre_submit: None,
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
                let button = DynamicButton::new(
                    new_genre.clone(),
                    MetadataActions::RemoveGenre { id: new_genre },
                );
                this.inner.inner.inner.data.genres.push(Genre { button });
                render_flag.set();
                Ok(None)
            }
        }
    }
}
