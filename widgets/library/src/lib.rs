use std::{fmt::Debug, ops::ControlFlow, pin::pin, rc::Rc};

use jellyfin::{
    JellyfinClient, JellyfinVec,
    connect::JsonResponseHelper,
    items::{ALL_FIELDS, GetItemsQuery, MediaItem},
    user_views::UserView,
};
use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::UserViewCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, ImageCache, Picker, Stats};
use jellyhaj_event_listener::JellyfinEventInterests;
use jellyhaj_item_grid::{GridWrapper, ItemGrid, ItemGridAction, new_item_grid};
use jellyhaj_keybinds_widget::{KeybindWidget, KeybindWrapper};
use jellyhaj_widgets_core::{
    ContextRef, GetFromContext, ItemWidget, JellyhajWidgetBase, RenderFlag, Result, WidgetContext,
    Wrapper,
    async_task::{Cancellation, Cancelled, StreamExt, UnboundedSender},
    mapper::{ActionMapper, ActionMapperBase, ActionMapperWidget},
    outer::{Named, UnwrapWidget},
    spawn::tracing::{debug, info_span},
    valuable::Valuable,
};
use spawn::Spawner;
use sqlx::SqliteConnection;

type DB = Rc<tokio::sync::Mutex<SqliteConnection>>;

#[derive(Debug)]
pub enum LibraryAction {
    Reload,
    Remove,
    Add(Vec<MediaItem>),
}

pub struct KeybindMapper {
    view: UserView,
}

impl CommandMapper<UserViewCommand> for KeybindMapper {
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

#[must_use]
pub fn make_item_query(seen: u32, parent: &str) -> GetItemsQuery<'_> {
    GetItemsQuery {
        start_index: seen.into(),
        limit: 10.into(),
        parent_id: parent.into(),
        enable_images: true.into(),
        enable_image_types: "Thumb, Backdrop, Primary".into(),
        enable_user_data: true.into(),
        sort_by: "DateLastContentAdded".into(),
        sort_order: "Descending".into(),
        fields: Some(ALL_FIELDS),
        ..Default::default()
    }
}

async fn fetch_library_content<W: Wrapper<LibraryAction>>(
    jellyfin: JellyfinClient,
    library_id: String,
    wrapper: W,
    sender: UnboundedSender<Result<W::F>>,
    cancel: Cancellation,
    seen: u32,
) {
    let inner = async move {
        let mut stream = pin!(JellyfinVec::stream_from(
            async |seen| {
                jellyfin
                    .get_items(&make_item_query(seen, &library_id))
                    .deserialize()
                    .await
            },
            seen
        ));
        while let Some(v) = stream.next().await {
            if sender
                .send(v.map(|v| wrapper.wrap(LibraryAction::Add(v.items))))
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

pub fn new_library_widget(
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
) -> LibraryWidget {
    let inner = new_item_grid(
        items.into_iter().map(|i| Entry::new(i, cx)).collect(),
        view.name.clone(),
        cx,
    );
    let inner = KeybindWidget::new(
        inner,
        Config::get_ref(cx).keybinds.user_view.clone(),
        KeybindMapper {
            view: UserView::clone(&view),
        },
    );
    let inner = UnwrapWidget::new(inner);
    LibraryWidget::new(
        inner,
        LibraryMapper {
            user_view: *view,
            seen,
        },
    )
}

#[derive(Valuable)]
pub struct LibraryMapper {
    user_view: UserView,
    seen: Option<u32>,
}

type InnerWidget = UnwrapWidget<KeybindWidget<UserViewCommand, ItemGrid<Entry>, KeybindMapper>>;

impl ActionMapperBase<InnerWidget> for LibraryMapper {
    type Action = LibraryAction;
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
> ActionMapper<R, InnerWidget> for LibraryMapper
{
    fn init(
        &mut self,
        _: &mut InnerWidget,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        _: WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
    ) {
        JellyfinEventInterests::get_ref(cx.refs).with(|events| {
            events.register_folder_modified(
                self.user_view.id.clone(),
                cx.submitter.wrap_with(|_| LibraryAction::Reload),
            );
            events.register_item_removed(
                self.user_view.id.clone(),
                cx.submitter.wrap_with(|_| LibraryAction::Remove),
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

    fn map_action(
        &mut self,
        this: &mut InnerWidget,
        cx: WidgetContext<'_, Self::Action, impl Wrapper<Self::Action>, R>,
        this_cx: WidgetContext<
            '_,
            <InnerWidget as JellyhajWidgetBase>::Action,
            impl Wrapper<<InnerWidget as JellyhajWidgetBase>::Action>,
            R,
        >,
        action: Self::Action,
        render_flag: &mut RenderFlag,
    ) -> Result<Option<<InnerWidget as JellyhajWidgetBase>::ActionResult>> {
        match action {
            LibraryAction::Reload => Ok(Some(Navigation::Replace(NextScreen::LoadUserView(
                Box::new(self.user_view.clone()),
            )))),
            LibraryAction::Remove => Ok(Some(Navigation::PopContext)),
            LibraryAction::Add(items) => {
                debug!("received {} additional items", items.len());
                render_flag.set();
                let start = this.inner.inner.len();
                this.inner
                    .inner
                    .extend(items.into_iter().enumerate().map(|(i, item)| {
                        let mut entry = Entry::new(item, cx.refs);
                        entry.init(this_cx.wrap_with(KeybindWrapper).wrap_with(GridWrapper {
                            index: start.strict_add(i),
                        }));
                        entry
                    }));
                Ok(None)
            }
        }
    }
}

pub struct LibraryName;
impl Named for LibraryName {
    const NAME: &str = "library";
}

pub type LibraryWidget = ActionMapperWidget<LibraryName, InnerWidget, LibraryMapper>;
