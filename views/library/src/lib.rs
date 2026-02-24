use std::{ops::ControlFlow, pin::Pin};

use color_eyre::{Result, eyre::Context};
use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
    user_views::UserView,
};
use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::UserViewCommand,
    render::{NavigationResult, render_widget},
    state::{Navigation, Next, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryState};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_item_grid::{ItemGridAction, ItemGridData};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_widgets_core::outer::{Named, OuterState};

async fn fetch_user_view(jellyfin: JellyfinClient, view: UserView) -> Result<Next> {
    let user_id = jellyfin.get_auth().user.id.as_str();
    let items = JellyfinVec::collect(async |start| {
        jellyfin
            .get_items(&GetItemsQuery {
                user_id: user_id.into(),
                start_index: start.into(),
                limit: 100.into(),
                recursive: None,
                parent_id: view.id.as_str().into(),
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
            .await
            .context("deserializing items")
    })
    .await?;
    Ok(Box::new(NextScreen::UserView { view, items }))
}

pub fn render_fetch_user_view(
    cx: Pin<&mut TuiContext>,
    view: UserView,
) -> impl Future<Output = NavigationResult> {
    let title = format!("Loading user view {}", view.name);
    let inner = fetch_user_view(cx.jellyfin.clone(), view);
    make_fetch(cx, title, inner)
}

#[derive(Debug)]
pub enum Pass {
    Reload,
    Quit,
}

struct Mapper {
    view: UserView,
}

impl CommandMapper<UserViewCommand> for Mapper {
    type A = ItemGridAction<EntryAction>;

    fn map(&self, command: UserViewCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            UserViewCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            UserViewCommand::Reload => ControlFlow::Break(Navigation::Replace(Box::new(
                NextScreen::LoadUserView(self.view.clone()),
            ))),
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
struct Name;
impl Named for Name {
    const NAME: &str = "library";
}

pub fn render_user_view(
    mut cx: Pin<&mut TuiContext>,
    view: UserView,
    items: Vec<MediaItem>,
) -> impl Future<Output = NavigationResult> {
    let inner = ItemGridData::<Entry>::new(
        items
            .into_iter()
            .map(|i| EntryState::new(i, cx.as_mut()))
            .collect(),
        view.name.clone(),
        0,
    );
    let inner = KeybindState::new(
        inner,
        cx.config.help_prefixes.clone(),
        cx.config.keybinds.user_view.clone(),
        Mapper { view },
    );
    let state = OuterState::<Name, _, _, _>::new(inner);
    render_widget(cx, state)
}
