use std::{fmt::Debug, ops::ControlFlow};

use jellyfin::{JellyfinClient, items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    CommandMapper, Config,
    context::{DB, JellyfinEventInterests, Spawner},
    keybinds::HomeScreenCommand,
    render::KeybindAction,
    state::{Navigation, NextScreen, flatten_control_flow},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryState, ImageProtocolCache, Picker, Stats};
use jellyhaj_item_screen::{ItemListState, ItemScreen, ItemScreenAction, ItemScreenState};
use jellyhaj_keybinds_widget::{KeybindState, KeybindWidget};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, JellyhajWidgetState, Result, WidgetContext, Wrapper,
};

#[derive(Debug)]
pub enum HomeScreenAction {
    Inner(ItemScreenAction<EntryAction>),
    Reload,
}

#[derive(Clone, Copy)]
struct Mapper;
impl CommandMapper<HomeScreenCommand> for Mapper {
    type A = ItemScreenAction<EntryAction>;

    fn map(&self, command: HomeScreenCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            HomeScreenCommand::Quit => ControlFlow::Break(Navigation::Exit),
            HomeScreenCommand::Reload => {
                ControlFlow::Break(Navigation::Replace(Box::new(NextScreen::LoadHomeScreen)))
            }
            HomeScreenCommand::Left => ControlFlow::Continue(ItemScreenAction::Left),
            HomeScreenCommand::Right => ControlFlow::Continue(ItemScreenAction::Right),
            HomeScreenCommand::Up => ControlFlow::Continue(ItemScreenAction::Up),
            HomeScreenCommand::Down => ControlFlow::Continue(ItemScreenAction::Down),
            HomeScreenCommand::Entry(entry_command) => ControlFlow::Continue(
                ItemScreenAction::CurrentInner(EntryAction::Command(entry_command)),
            ),
            HomeScreenCommand::Global(global_show) => ControlFlow::Break(global_show.into()),
        }
    }
}
impl Wrapper<KeybindAction<ItemScreenAction<EntryAction>>> for Mapper {
    type F = KeybindAction<HomeScreenAction>;

    fn wrap(&self, val: KeybindAction<ItemScreenAction<EntryAction>>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(HomeScreenAction::Inner(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}
impl Wrapper<String> for Mapper {
    type F = KeybindAction<HomeScreenAction>;

    fn wrap(&self, _: String) -> Self::F {
        KeybindAction::Inner(HomeScreenAction::Reload)
    }
}

pub struct HomeScreenState<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> {
    inner: KeybindState<R, HomeScreenCommand, ItemScreenState<R, EntryState>, Mapper>,
    register: Option<Vec<String>>,
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> Debug for HomeScreenState<R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HomeScreenState")
            .field("inner", &self.inner)
            .field("register", &self.register)
            .finish()
    }
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + ContextRef<ImageProtocolCache>
        + 'static,
> HomeScreenState<R>
{
    pub fn new(
        cx: &R,
        cont: Vec<MediaItem>,
        next_up: Vec<MediaItem>,
        libraries: Vec<UserView>,
        library_latest: Vec<(String, Vec<MediaItem>)>,
    ) -> Self {
        let register: Vec<_> = libraries.iter().map(|l| l.id.clone()).collect();
        let screen = ItemScreenState::new(
            [
                ItemListState::new(
                    cont.into_iter().map(|i| EntryState::new(i, cx)),
                    "Continue Watching".to_string(),
                ),
                ItemListState::new(
                    next_up.into_iter().map(|i| EntryState::new(i, cx)),
                    "Next Up".to_string(),
                ),
                ItemListState::new(
                    libraries.into_iter().map(|i| EntryState::new(i, cx)),
                    "Continue Watching".to_string(),
                ),
            ]
            .into_iter()
            .chain(library_latest.into_iter().map(|(title, list)| {
                ItemListState::new(list.into_iter().map(|i| EntryState::new(i, cx)), title)
            }))
            .filter(|l| !l.items.is_empty())
            .collect(),
            "Home".to_string(),
        );
        let inner = KeybindState::new(
            screen,
            Config::get_ref(cx).keybinds.home_screen.clone(),
            Mapper,
        );
        Self {
            inner,
            register: Some(register),
        }
    }
}

pub struct HomeScreen<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> {
    inner: KeybindWidget<R, HomeScreenCommand, ItemScreen<R, Entry>, Mapper>,
    register: Option<Vec<String>>,
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> JellyhajWidgetState<R> for HomeScreenState<R>
{
    type Action = KeybindAction<HomeScreenAction>;

    type ActionResult = Navigation;

    type Widget = HomeScreen<R>;

    const NAME: &str = "home-screen";

    fn visit_children(visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor
            .visit::<R, KeybindState<R, HomeScreenCommand, ItemScreenState<R, EntryState>, Mapper>>(
            );
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        HomeScreen {
            inner: self.inner.into_widget(cx),
            register: self.register,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl jellyhaj_widgets_core::Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let action = match action {
            KeybindAction::Inner(HomeScreenAction::Reload) => {
                return Ok(Some(Navigation::Replace(Box::new(
                    NextScreen::LoadHomeScreen,
                ))));
            }
            KeybindAction::Inner(HomeScreenAction::Inner(a)) => KeybindAction::Inner(a),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        };
        flatten_control_flow(self.inner.apply_action(cx.wrap_with(Mapper), action))
    }
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> JellyhajWidget<R> for HomeScreen<R>
{
    type Action = KeybindAction<HomeScreenAction>;

    type ActionResult = Navigation;

    type State = HomeScreenState<R>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        HomeScreenState {
            inner: self.inner.into_state(),
            register: self.register,
        }
    }

    fn accepts_text_input(&self) -> bool {
        self.inner.accepts_text_input()
    }

    fn accept_char(&mut self, text: char) {
        self.inner.accept_char(text);
    }

    fn accept_text(&mut self, text: String) {
        self.inner.accept_text(text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let action = match action {
            KeybindAction::Inner(HomeScreenAction::Reload) => {
                return Ok(Some(Navigation::Replace(Box::new(
                    NextScreen::LoadHomeScreen,
                ))));
            }
            KeybindAction::Inner(HomeScreenAction::Inner(a)) => KeybindAction::Inner(a),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        };
        flatten_control_flow(self.inner.apply_action(cx.wrap_with(Mapper), action))
    }

    fn click(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        position: jellyhaj_widgets_core::Position,
        size: jellyhaj_widgets_core::Size,
        kind: jellyhaj_widgets_core::MouseEventKind,
        modifier: jellyhaj_widgets_core::KeyModifiers,
    ) -> Result<Option<Self::ActionResult>> {
        flatten_control_flow(
            self.inner
                .click(cx.wrap_with(Mapper), position, size, kind, modifier),
        )
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        if let Some(register) = self.register.take() {
            let mut interests = JellyfinEventInterests::get_ref(cx.refs).get();
            let submitter = cx.submitter.wrap_with(Mapper);
            for item in register {
                interests.register_folder_modified(item, submitter);
            }
        }
        self.inner
            .render_fallible_inner(area, buf, cx.wrap_with(Mapper))
    }
}
