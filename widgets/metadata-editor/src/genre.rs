use std::{convert::Infallible, sync::Arc};

use jellyhaj_core::{
    Config,
    keybinds::FormCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_form_widget::{
    FormAction,
    button::Button,
    form::{
        Form, FormCommandMapper, FormResultMapper,
        component::{ComponentVec, FormComponent},
    },
    form_component, form_widget,
    label::DynamicLabel,
    selection::DynamicSelection,
    seperator::Seperator,
    text_field::TextField,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, JellyhajWidgetBase, Result, Wrapper,
    async_task::ErasedSubmitter,
    mapper::{ActionMapper, ActionMapperBase},
    outer::UnwrapWidget,
};
use valuable::Valuable;

pub struct GenreMapper;

impl FormResultMapper<GenreSelection> for GenreMapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<GenreSelection>,
        form_result: <GenreSelection as FormComponent>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            FormAction<<GenreSelection as FormComponent>::Action>,
            impl Wrapper<FormAction<<GenreSelection as FormComponent>::Action>>,
            (),
        >,
        _render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<Self::Res>> {
        match form_result {
            GenreDo::Add => Ok(Some(Navigation::Push(NextScreen::NewGenre(
                state
                    .data
                    .new_genre_sender
                    .as_ref()
                    .expect("new_genre_sender was not populated")
                    .clone(),
            )))),
            GenreDo::Select => {
                state
                    .data
                    .result_sender
                    .spawn_value_infallible(state.data.existing.get().clone());
                Ok(Some(Navigation::PopContext))
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum GenreDo {
    Add,
    Select,
}

impl From<Infallible> for GenreDo {
    fn from(value: Infallible) -> Self {
        match value {}
    }
}

#[form_component(GenreDo)]
#[derive(Debug, Valuable)]
pub struct CurrentGenre {
    #[form(descr = "")]
    pub name: DynamicLabel,
}

#[form_widget("Add new genre", GenreDo, GenreMapper)]
#[derive(Debug, Valuable)]
pub struct GenreSelection {
    #[form(descr = "Select exoisting genre")]
    existing: DynamicSelection,
    #[form(descr = "Add selected")]
    add: Button<GenreDo>,
    #[form(descr = "New Genre")]
    select: Button<GenreDo>,
    #[form(descr = "Current Genres")]
    current_sep: Seperator,
    #[form(flatten)]
    current: ComponentVec<CurrentGenre>,
    #[form(skip)]
    #[valuable(skip)]
    new_genre_sender: Option<Arc<dyn ErasedSubmitter<String>>>,
    #[form(skip)]
    #[valuable(skip)]
    result_sender: Arc<dyn ErasedSubmitter<String>>,
}

impl GenreSelection {
    pub fn new(
        selected: Vec<String>,
        all_genres: Vec<String>,
        result_sender: Arc<dyn ErasedSubmitter<String>>,
    ) -> Self {
        Self {
            existing: DynamicSelection::new(all_genres),
            add: Button::new(GenreDo::Select),
            select: Button::new(GenreDo::Add),
            current_sep: Seperator,
            current: ComponentVec::new_with(selected.into_iter().map(|genre| CurrentGenre {
                name: DynamicLabel::new(genre),
            })),
            new_genre_sender: None,
            result_sender,
        }
    }
}

pub struct AddMapper;

impl FormResultMapper<AddGenre> for AddMapper {
    type Res = Navigation;

    fn map(
        state: &mut Form<AddGenre>,
        form_result: <AddGenre as FormComponent>::AR,
        _cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            FormAction<<AddGenre as FormComponent>::Action>,
            impl Wrapper<FormAction<<AddGenre as FormComponent>::Action>>,
            (),
        >,
        _render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<Self::Res>> {
        let Add::Add = form_result;
        state
            .data
            .new_genre_sender
            .spawn_value_infallible(state.data.name.text.clone());
        Ok(Some(Navigation::PopContext))
    }
}

type InnerWidget = UnwrapWidget<
    KeybindWidget<
        FormCommand,
        UnwrapWidget<GenreSelectionWidget>,
        FormCommandMapper<GenreSelectionAction>,
    >,
>;

#[derive(Valuable)]
pub struct GenreSelectionActionMapper;
impl ActionMapperBase<InnerWidget> for GenreSelectionActionMapper {
    type Action = String;
}

impl<R: ContextRef<Config> + 'static> ActionMapper<R, InnerWidget> for GenreSelectionActionMapper {
    fn init(
        &mut self,
        this: &mut InnerWidget,
        cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
    ) {
        this.inner.inner.inner.data.new_genre_sender = Some(Arc::new(cx.submitter.erased()));
    }

    fn map_action(
        &mut self,
        this: &mut InnerWidget,
        _cx: jellyhaj_widgets_core::WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _this_cx: jellyhaj_widgets_core::WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
        action: Self::Action,
        render_flag: &mut jellyhaj_widgets_core::RenderFlag,
    ) -> Result<Option<<InnerWidget as JellyhajWidgetBase>::ActionResult>> {
        this.inner
            .inner
            .inner
            .data
            .existing
            .add_and_set_option(action);
        render_flag.set();
        this.inner.inner.inner.sel = GenreSelectionSelection::Select(());
        Ok(None)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Add {
    Add,
}

impl From<Infallible> for Add {
    fn from(_value: Infallible) -> Self {
        unreachable!()
    }
}

#[form_widget("Add new genre", Add, AddMapper)]
#[derive(Debug, Valuable)]
pub struct AddGenre {
    #[form(descr = "Genre name")]
    name: TextField,
    #[form(descr = "Add")]
    add: Button<Add>,
    #[form(skip)]
    #[valuable(skip)]
    new_genre_sender: Arc<dyn ErasedSubmitter<String>>,
}

impl AddGenre {
    pub fn new(new_genre_sender: Arc<dyn ErasedSubmitter<String>>) -> Self {
        Self {
            name: TextField::new(String::new()),
            add: Button::new(Add::Add),
            new_genre_sender,
        }
    }
}
