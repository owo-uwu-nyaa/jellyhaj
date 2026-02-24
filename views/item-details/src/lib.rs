use std::{ops::ControlFlow, pin::Pin};

use color_eyre::eyre::Context;
use jellyfin::items::MediaItem;
use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::{ItemDetailsCommand, ItemListDetailsCommand},
    render::{NavigationResult, render_widget},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::EntryAction;
use jellyhaj_fetch_view::{fetch_all_children, fetch_child_of_type, fetch_item, make_fetch};
use jellyhaj_item_details_widget::{
    DisplayAction, DisplayListAction, ItemDisplayState, ItemListDisplayState,
};
use jellyhaj_item_list::ItemListAction;
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_widgets_core::outer::{Named, OuterState};
use tokio::try_join;
use tracing::instrument;

#[instrument(skip_all)]
pub fn render_fetch_episode(
    cx: Pin<&mut TuiContext>,
    parent: String,
) -> impl Future<Output = NavigationResult> {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let item = fetch_child_of_type(&jellyfin, "Episode, Movie, Music", &parent)
            .await
            .context("fetching episode")?;
        Ok(Box::new(NextScreen::ItemDetails(item)))
    };
    make_fetch(cx, "Fetching single item", fut)
}

#[instrument(skip_all)]
pub fn render_fetch_item_list(
    cx: Pin<&mut TuiContext>,
    parent: MediaItem,
) -> impl Future<Output = NavigationResult> {
    let jellyfin = cx.jellyfin.clone();
    let title = format!("Loading {}", &parent.name);
    let fut = async move {
        let items = fetch_all_children(&jellyfin, &parent.id).await?;
        Ok(Box::new(NextScreen::ItemListDetails(parent, items)))
    };
    make_fetch(cx, title, fut)
}

#[instrument(skip_all)]
pub fn render_fetch_item_list_ref(
    cx: Pin<&mut TuiContext>,
    parent: String,
) -> impl Future<Output = NavigationResult> {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let (parent, children) = try_join!(
            fetch_item(&jellyfin, &parent),
            fetch_all_children(&jellyfin, &parent)
        )?;
        Ok(Box::new(NextScreen::ItemListDetails(parent, children)))
    };
    make_fetch(cx, "Loading item list", fut)
}

struct DetailsMapper {
    id: String,
}

impl CommandMapper<ItemDetailsCommand> for DetailsMapper {
    type A = DisplayAction;

    fn map(&self, command: ItemDetailsCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            ItemDetailsCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            ItemDetailsCommand::Up => ControlFlow::Continue(DisplayAction::Up),
            ItemDetailsCommand::Down => ControlFlow::Continue(DisplayAction::Down),
            ItemDetailsCommand::Reload => ControlFlow::Break(Navigation::Replace(Box::new(
                NextScreen::FetchItemDetails(self.id.clone()),
            ))),
            ItemDetailsCommand::Entry(entry_command) => {
                ControlFlow::Continue(DisplayAction::Inner(EntryAction::Command(entry_command)))
            }
            ItemDetailsCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct DetailsName;

impl Named for DetailsName {
    const NAME: &str = "item-details";
}

#[instrument(skip_all)]
pub fn render_item_details(
    mut cx: Pin<&mut TuiContext>,
    item: MediaItem,
) -> impl Future<Output = NavigationResult> {
    let id = item.id.clone();
    let state = ItemDisplayState::new(item, cx.as_mut());
    let state = KeybindState::new(
        state,
        cx.config.help_prefixes.clone(),
        cx.config.keybinds.item_details.clone(),
        DetailsMapper { id },
    );
    render_widget(cx, OuterState::<DetailsName, _, _, _>::new(state))
}

struct ListMapper {
    id: String,
}

impl CommandMapper<ItemListDetailsCommand> for ListMapper {
    type A = DisplayListAction;

    fn map(&self, command: ItemListDetailsCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            ItemListDetailsCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            ItemListDetailsCommand::Reload => ControlFlow::Break(Navigation::Replace(Box::new(
                NextScreen::FetchItemListDetailsRef(self.id.clone()),
            ))),
            ItemListDetailsCommand::Up => ControlFlow::Continue(DisplayListAction::Up),
            ItemListDetailsCommand::Down => ControlFlow::Continue(DisplayListAction::Down),
            ItemListDetailsCommand::Left => ControlFlow::Continue(DisplayListAction::Left),
            ItemListDetailsCommand::Right => ControlFlow::Continue(DisplayListAction::Right),
            ItemListDetailsCommand::Entry(entry_command) => {
                ControlFlow::Continue(DisplayListAction::Inner(ItemListAction::CurrentInner(
                    EntryAction::Command(entry_command),
                )))
            }
            ItemListDetailsCommand::RefreshParentItem => ControlFlow::Break(Navigation::Push(
                Box::new(NextScreen::RefreshItem(self.id.clone())),
            )),
            ItemListDetailsCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct ListName;

impl Named for ListName {
    const NAME: &str = "item-list-details";
}

#[instrument(skip_all)]
pub fn render_item_list_details(
    mut cx: Pin<&mut TuiContext>,
    parent: MediaItem,
    children: Vec<MediaItem>,
) -> impl Future<Output = NavigationResult> {
    let id = parent.id.clone();
    let state = ItemListDisplayState::new(children, parent, cx.as_mut());
    let state = KeybindState::new(
        state,
        cx.config.help_prefixes.clone(),
        cx.config.keybinds.item_list_details.clone(),
        ListMapper { id },
    );
    render_widget(cx, OuterState::<ListName, _, _, _>::new(state))
}
