use std::{fmt::Debug, ops::ControlFlow, pin::pin};

use color_eyre::eyre::Context;
use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
    user_views::UserView,
};
use jellyhaj_core::{
    CommandMapper, Config,
    context::{DB, JellyfinEventInterests, Spawner},
    keybinds::UserViewCommand,
    render::KeybindAction,
    state::{Navigation, NextScreen, flatten_control_flow},
};
use jellyhaj_entry_widget::{Entry, EntryAction, ImageCache, Picker, Stats};
use jellyhaj_item_grid::{GridWrapper, ItemGrid, ItemGridAction, new_item_grid};
use jellyhaj_keybinds_widget::{KeybindWidget, KeybindWrapper};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, ItemWidget, JellyhajWidget, JellyhajWidgetBase, Result,
    WidgetContext, WidgetTreeVisitor, Wrapper,
    async_task::{Cancellation, Cancelled, Sender, SinkExt, StreamExt},
    spawn::tracing::{debug, info_span},
    valuable::Valuable,
};

#[derive(Debug)]
pub enum LibraryAction {
    Inner(ItemGridAction<EntryAction>),
    Reload,
    Remove,
    Add(Vec<MediaItem>),
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

async fn fetch_library_content<W: Wrapper<KeybindAction<LibraryAction>>>(
    jellyfin: JellyfinClient,
    library_id: String,
    wrapper: W,
    mut sender: Sender<Result<W::F>>,
    cancel: Cancellation,
    seen: u32,
) {
    let inner = async move {
        let mut stream = pin!(JellyfinVec::stream_from(
            async |seen| {
                let user_id = jellyfin.get_auth().user.id.as_str();
                jellyfin
                    .get_items(&GetItemsQuery {
                        user_id: user_id.into(),
                        start_index: seen.into(),
                        limit: 100.into(),
                        recursive: None,
                        parent_id: library_id.as_str().into(),
                        exclude_item_types: None,
                        include_item_types: None,
                        enable_images: true.into(),
                        enable_image_types: "Thumb, Backdrop, Primary".into(),
                        image_type_limit: 1.into(),
                        enable_user_data: true.into(),
                        fields: None,
                        sort_by: "DateLastContentAdded".into(),
                        sort_order: "Descending".into(),
                    })
                    .await
                    .context("requesting items")?
                    .deserialize()
                    .context("deserializing items")
            },
            seen
        ));
        while let Some(v) = stream.next().await {
            if sender
                .feed(v.map(|v| wrapper.wrap(KeybindAction::Inner(LibraryAction::Add(v.items)))))
                .await
                .is_err()
            {
                break;
            }
        }
    };
    Cancelled {
        f: inner,
        cancel: cancel.cancelled(),
    }
    .await;
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
             + ContextRef<ImageCache>
             + 'static
         ),
        seen: Option<u32>,
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
            seen,
        }
    }
}

#[derive(Valuable)]
pub struct LibraryWidget {
    #[valuable(skip)]
    inner: KeybindWidget<UserViewCommand, ItemGrid<Entry>, Mapper>,
    user_view: UserView,
    seen: Option<u32>,
}

impl JellyhajWidgetBase for LibraryWidget {
    type Action = KeybindAction<LibraryAction>;

    type ActionResult = Navigation;

    const NAME: &str = "library";

    fn visit_children(&self, visitor: &mut impl WidgetTreeVisitor) {
        visitor.visit(&self.inner);
    }

    fn min_width(&self) -> Option<u16> {
        self.inner.min_width()
    }
    fn min_height(&self) -> Option<u16> {
        self.inner.min_height()
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
}

impl<
    R: ContextRef<Spawner>
        + ContextRef<Config>
        + ContextRef<Picker>
        + ContextRef<Stats>
        + ContextRef<JellyfinClient>
        + ContextRef<JellyfinEventInterests>
        + ContextRef<DB>
        + ContextRef<ImageCache>
        + 'static,
> JellyhajWidget<R> for LibraryWidget
{
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
            KeybindAction::Inner(LibraryAction::Add(items)) => {
                debug!("received {} additional items", items.len());
                let start = self.inner.inner.len();
                self.inner
                    .inner
                    .extend(items.into_iter().enumerate().map(|(i, item)| {
                        let mut entry = Entry::new(item, cx.refs);
                        entry.init(cx.wrap_with(Wrap).wrap_with(KeybindWrapper).wrap_with(
                            GridWrapper {
                                index: start.strict_add(i),
                            },
                        ));
                        entry
                    }));
                return Ok(None);
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
        if let Some(seen) = self.seen.take() {
            let jellyfin = JellyfinClient::get_ref(cx.refs).clone();
            let id = self.user_view.id.clone();
            cx.submitter.spawn(
                fetch_library_content(
                    jellyfin,
                    id,
                    cx.submitter.wrapper(),
                    cx.submitter.sender().clone(),
                    cx.submitter.cancel_token().clone(),
                    seen,
                ),
                info_span!("fetch_library_add"),
                "fetch_library_add",
            );
        }
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
