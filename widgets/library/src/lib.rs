use std::{fmt::Debug, ops::ControlFlow};

use jellyfin::{JellyfinClient, items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    CommandMapper, Config,
    context::{DB, JellyfinEventInterests, Spawner},
    keybinds::UserViewCommand,
    render::KeybindAction,
    state::{Navigation, NextScreen, flatten_control_flow},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryState, ImageProtocolCache, Picker, Stats};
use jellyhaj_item_grid::{ItemGrid, ItemGridAction, ItemGridState};
use jellyhaj_keybinds_widget::{KeybindState, KeybindWidget};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, JellyhajWidgetState, Result, WidgetContext,
    WidgetTreeVisitor, Wrapper,
};

#[derive(Debug)]
pub enum LibraryAction {
    Inner(ItemGridAction<EntryAction>),
    Reload,
    Remove,
}

struct Mapper {
    view: UserView,
}

impl CommandMapper<UserViewCommand> for Mapper {
    type A = ItemGridAction<EntryAction>;

    fn map(&self, command: UserViewCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            UserViewCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            UserViewCommand::Reload => ControlFlow::Break(Navigation::Replace(
                NextScreen::LoadUserView(Box::new(self.view.clone())),
            )),
            UserViewCommand::Prev => ControlFlow::Continue(ItemGridAction::Left),
            UserViewCommand::Next => ControlFlow::Continue(ItemGridAction::Right),
            UserViewCommand::Up => ControlFlow::Continue(ItemGridAction::Up),
            UserViewCommand::Down => ControlFlow::Continue(ItemGridAction::Down),
            UserViewCommand::Entry(entry_command) => ControlFlow::Continue(
                ItemGridAction::CurrentInner(EntryAction::Command(entry_command)),
            ),
            UserViewCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

#[derive(Clone, Copy)]
struct W;
impl Wrapper<KeybindAction<ItemGridAction<EntryAction>>> for W {
    type F = KeybindAction<LibraryAction>;

    fn wrap(&self, val: KeybindAction<ItemGridAction<EntryAction>>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(LibraryAction::Inner(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}

pub struct LibraryState<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> {
    inner: KeybindState<R, UserViewCommand, ItemGridState<R, EntryState>, Mapper>,
    user_view: UserView,
    registered: bool,
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
> Debug for LibraryState<R>
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LibraryState")
            .field("inner", &self.inner)
            .field("user_view", &self.user_view)
            .field("registered", &self.registered)
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
> LibraryState<R>
{
    pub fn new(view: Box<UserView>, items: Vec<MediaItem>, cx: &R) -> Self {
        let inner = ItemGridState::<R, EntryState>::new(
            items.into_iter().map(|i| EntryState::new(i, cx)).collect(),
            view.name.clone(),
            0,
        );
        let inner = KeybindState::new(
            inner,
            Config::get_ref(cx).keybinds.user_view.clone(),
            Mapper {
                view: UserView::clone(&view),
            },
        );

        Self {
            inner,
            user_view: *view,
            registered: false,
        }
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
> JellyhajWidgetState<R> for LibraryState<R>
{
    type Action = KeybindAction<LibraryAction>;

    type ActionResult = Navigation;

    type Widget = LibraryWidget<R>;

    const NAME: &str = "library";

    fn visit_children(visitor: &mut impl WidgetTreeVisitor) {
        visitor
            .visit::<R, KeybindState<R, UserViewCommand, ItemGridState<R, EntryState>, Mapper>>();
    }

    fn into_widget(self, cx: &R) -> Self::Widget {
        LibraryWidget {
            inner: self.inner.into_widget(cx),
            user_view: self.user_view,
            registered: self.registered,
        }
    }

    fn apply_action(
        &mut self,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        action: Self::Action,
    ) -> Result<Option<Self::ActionResult>> {
        let action = match action {
            KeybindAction::Inner(LibraryAction::Reload) => {
                return Ok(Some(Navigation::Replace(NextScreen::LoadUserView(
                    Box::new(self.user_view.clone()),
                ))));
            }
            KeybindAction::Inner(LibraryAction::Remove) => {
                return Ok(Some(Navigation::PopContext));
            }
            KeybindAction::Inner(LibraryAction::Inner(action)) => KeybindAction::Inner(action),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        };
        flatten_control_flow(self.inner.apply_action(cx.wrap_with(W), action))
    }
}

pub struct LibraryWidget<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + 'static,
> {
    inner: KeybindWidget<R, UserViewCommand, ItemGrid<R, Entry>, Mapper>,
    user_view: UserView,
    registered: bool,
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
> JellyhajWidget<R> for LibraryWidget<R>
{
    type Action = KeybindAction<LibraryAction>;

    type ActionResult = Navigation;

    type State = LibraryState<R>;

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }

    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
    }

    fn into_state(self) -> Self::State {
        LibraryState {
            inner: self.inner.into_state(),
            user_view: self.user_view,
            registered: self.registered,
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
            KeybindAction::Inner(LibraryAction::Reload) => {
                return Ok(Some(Navigation::Replace(NextScreen::LoadUserView(
                    Box::new(self.user_view.clone()),
                ))));
            }
            KeybindAction::Inner(LibraryAction::Remove) => {
                return Ok(Some(Navigation::PopContext));
            }
            KeybindAction::Inner(LibraryAction::Inner(action)) => KeybindAction::Inner(action),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        };
        flatten_control_flow(self.inner.apply_action(cx.wrap_with(W), action))
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
                .click(cx.wrap_with(W), position, size, kind, modifier),
        )
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        if !self.registered {
            self.registered = true;
            let mut events = JellyfinEventInterests::get_ref(cx.refs).get();
            events.register_folder_modified(
                self.user_view.id.clone(),
                cx.submitter
                    .wrap_with(|_| KeybindAction::Inner(LibraryAction::Reload)),
            );
            events.register_item_removed(
                self.user_view.id.clone(),
                cx.submitter
                    .wrap_with(|_| KeybindAction::Inner(LibraryAction::Remove)),
            );
        }
        self.inner.render_fallible_inner(area, buf, cx.wrap_with(W))
    }
}
