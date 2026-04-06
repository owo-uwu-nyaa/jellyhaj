use std::{fmt::Debug, ops::ControlFlow};

use jellyfin::{JellyfinClient, items::MediaItem, user_views::UserView};
use jellyhaj_core::{
    CommandMapper, Config,
    context::{DB, JellyfinEventInterests, Spawner},
    keybinds::UserViewCommand,
    render::KeybindAction,
    state::{Navigation, NextScreen, flatten_control_flow},
};
use jellyhaj_entry_widget::{Entry, EntryAction, ImageProtocolCache, Picker, Stats};
use jellyhaj_item_grid::{ItemGrid, ItemGridAction, new_item_grid};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, JellyhajWidget, Result, WidgetContext, WidgetTreeVisitor, Wrapper,
    valuable::Valuable,
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
struct Wrap;
impl Wrapper<KeybindAction<ItemGridAction<EntryAction>>> for Wrap {
    type F = KeybindAction<LibraryAction>;

    fn wrap(&self, val: KeybindAction<ItemGridAction<EntryAction>>) -> Self::F {
        match val {
            KeybindAction::Inner(v) => KeybindAction::Inner(LibraryAction::Inner(v)),
            KeybindAction::Key(key_event) => KeybindAction::Key(key_event),
        }
    }
}

impl LibraryWidget {
    pub fn new(
        view: Box<UserView>,
        items: Vec<MediaItem>,
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
    ) -> Self {
        let inner = new_item_grid(
            items.into_iter().map(|i| Entry::new(i, cx)).collect(),
            view.name.clone(),
            cx,
        );
        let inner = KeybindWidget::new(
            inner,
            Config::get_ref(cx).keybinds.user_view.clone(),
            Mapper {
                view: UserView::clone(&view),
            },
        );

        Self {
            inner,
            user_view: *view,
        }
    }
}

#[derive(Valuable)]
pub struct LibraryWidget {
    #[valuable(skip)]
    inner: KeybindWidget<UserViewCommand, ItemGrid<Entry>, Mapper>,
    user_view: UserView,
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
> JellyhajWidget<R> for LibraryWidget
{
    type Action = KeybindAction<LibraryAction>;

    type ActionResult = Navigation;

    const NAME: &str = "library";

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
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
        flatten_control_flow(self.inner.apply_action(cx.wrap_with(Wrap), action))
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
                .click(cx.wrap_with(Wrap), position, size, kind, modifier),
        )
    }

    fn init(&mut self, cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>) {
        JellyfinEventInterests::get_ref(cx.refs).with(|events| {
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
        });
    }

    fn render_fallible_inner(
        &mut self,
        area: jellyhaj_widgets_core::Rect,
        buf: &mut jellyhaj_widgets_core::Buffer,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
    ) -> Result<()> {
        self.inner
            .render_fallible_inner(area, buf, cx.wrap_with(Wrap))
    }
}
