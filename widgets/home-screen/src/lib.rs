use std::{fmt::Debug, ops::ControlFlow};

use jellyfin::{JellyfinClient, items::MediaItem, socket::ChangedUserData, user_views::UserView};
use jellyhaj_core::{
    CommandMapper, Config,
    context::{DB, JellyfinEventInterests, Spawner},
    keybinds::HomeScreenCommand,
    render::KeybindAction,
    state::{Navigation, NextScreen, flatten_control_flow},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryData, ImageProtocolCache, Picker, Stats};
use jellyhaj_item_screen::{ItemScreen, ItemScreenAction, new_item_list, new_item_screen};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, Result, WidgetContext, Wrapper,
};
use valuable::Valuable;

#[derive(Debug)]
pub enum HomeScreenAction {
    Inner(ItemScreenAction<EntryAction>),
    Reload,
    PotentialReload(bool),
}

#[derive(Clone, Copy)]
struct Mapper;
impl CommandMapper<HomeScreenCommand> for Mapper {
    type A = ItemScreenAction<EntryAction>;

    fn map(&self, command: HomeScreenCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            HomeScreenCommand::Quit => ControlFlow::Break(Navigation::Exit),
            HomeScreenCommand::Reload => {
                ControlFlow::Break(Navigation::Replace(NextScreen::LoadHomeScreen))
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
impl Wrapper<ChangedUserData> for Mapper {
    type F = KeybindAction<HomeScreenAction>;

    fn wrap(&self, val: ChangedUserData) -> Self::F {
        KeybindAction::Inner(HomeScreenAction::PotentialReload(
            val.user_data.playback_position_ticks == 0,
        ))
    }
}

impl HomeScreen {
    pub fn new(
        cx: &(
             impl ContextRef<Spawner>
             + ContextRef<Config>
             + ContextRef<Picker>
             + ContextRef<Stats>
             + ContextRef<JellyfinClient>
             + ContextRef<JellyfinEventInterests>
             + ContextRef<DB>
             + ContextRef<ImageProtocolCache>
             + 'static
         ),
        cont: Vec<MediaItem>,
        next_up: Vec<MediaItem>,
        libraries: Vec<UserView>,
        library_latest: Vec<(String, Vec<MediaItem>)>,
    ) -> Self {
        let screen = new_item_screen(
            [
                new_item_list(
                    cont.into_iter().map(|i| Entry::new(i, cx)),
                    "Continue Watching".to_string(),
                    cx,
                ),
                new_item_list(
                    next_up.into_iter().map(|i| Entry::new(i, cx)),
                    "Next Up".to_string(),
                    cx,
                ),
                new_item_list(
                    libraries.into_iter().map(|i| Entry::new(i, cx)),
                    "Continue Watching".to_string(),
                    cx,
                ),
            ]
            .into_iter()
            .chain(library_latest.into_iter().map(|(title, list)| {
                new_item_list(list.into_iter().map(|i| Entry::new(i, cx)), title, cx)
            }))
            .filter(|l| !l.is_empty()),
            "Home",
            cx,
        );
        let inner = KeybindWidget::new(
            screen,
            Config::get_ref(cx).keybinds.home_screen.clone(),
            Mapper,
        );
        Self { inner }
    }
}

#[derive(Valuable)]
pub struct HomeScreen {
    #[valuable(skip)]
    inner: KeybindWidget<HomeScreenCommand, ItemScreen<Entry>, Mapper>,
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
> JellyhajWidget<R> for HomeScreen
{
    type Action = KeybindAction<HomeScreenAction>;

    type ActionResult = Navigation;

    const NAME: &str = "home-screen";

    fn visit_children(&self, visitor: &mut impl jellyhaj_widgets_core::WidgetTreeVisitor) {
        visitor.visit::<R, _>(&self.inner);
    }

    fn min_width(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_width(&self.inner)
    }

    fn min_height(&self) -> Option<u16> {
        JellyhajWidget::<R>::min_height(&self.inner)
    }

    fn accepts_text_input(&self) -> bool {
        JellyhajWidget::<R>::accepts_text_input(&self.inner)
    }

    fn accept_char(&mut self, text: char) {
        JellyhajWidget::<R>::accept_char(&mut self.inner, text);
    }

    fn accept_text(&mut self, text: String) {
        JellyhajWidget::<R>::accept_text(&mut self.inner, text);
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let action = match action {
            KeybindAction::Inner(HomeScreenAction::Reload)
            | KeybindAction::Inner(HomeScreenAction::PotentialReload(true)) => {
                return Ok(Some(Navigation::Replace(NextScreen::LoadHomeScreen)));
            }
            KeybindAction::Inner(HomeScreenAction::PotentialReload(false)) => return Ok(None),
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

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        JellyfinEventInterests::get_ref(cx.refs).with(|interests| {
            for entry in self.inner.inner.iter().flat_map(|i| i.iter()) {
                match entry.data() {
                    EntryData::Item(item) => {
                        let submitter = cx.submitter.wrap_with(Mapper);
                        interests.register_changed_userdata(item.id.clone(), submitter);
                    }
                    EntryData::View(library) => {
                        let submitter = cx.submitter.wrap_with(Mapper);
                        interests.register_folder_modified(library.id.clone(), submitter);
                    }
                }
            }
        });
        self.inner.init(cx.wrap_with(Mapper));
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner
            .render_fallible_inner(area, buf, cx.wrap_with(Mapper))
    }
}
