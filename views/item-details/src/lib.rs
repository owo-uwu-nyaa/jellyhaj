use std::ops::ControlFlow;

use color_eyre::eyre::Context;
use jellyfin::items::MediaItem;
use jellyhaj_core::{
    CommandMapper,
    context::TuiContext,
    keybinds::{ItemDetailsCommand, ItemListDetailsCommand},
    render::{Erased, make_new_erased},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::EntryAction;
use jellyhaj_fetch_view::{fetch_all_children, fetch_child_of_type, fetch_item, make_fetch};
use jellyhaj_item_details_widget::{
    DisplayAction, ItemDetails,
    item_list_details::{ItemListDetailsAction, ItemListDetailsCommom},
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_tabs_widget::TabbedWidgets;
use jellyhaj_widgets_core::outer::{Named, OuterWidget};
use tokio::try_join;
use tracing::instrument;

#[instrument(skip_all)]
pub fn render_fetch_episode(cx: TuiContext, parent: String) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let item = fetch_child_of_type(&jellyfin, "Episode, Movie, Music", &parent)
            .await
            .context("fetching episode")?;
        Ok(NextScreen::ItemDetails(Box::new(item)))
    };
    make_fetch(cx, "Fetching single item", fut)
}

#[instrument(skip_all)]
pub fn render_fetch_item_list(cx: TuiContext, parent: Box<MediaItem>) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let title = format!("Loading {}", parent.name);
    let fut = async move {
        let items = fetch_all_children(&jellyfin, &parent.id).await?;
        Ok(NextScreen::ItemListDetails(parent, items))
    };
    make_fetch(cx, title, fut)
}

#[instrument(skip_all)]
pub fn render_fetch_item_list_ref(cx: TuiContext, parent: String) -> Erased {
    let jellyfin = cx.jellyfin.clone();
    let fut = async move {
        let (parent, children) = try_join!(
            fetch_item(&jellyfin, &parent),
            fetch_all_children(&jellyfin, &parent)
        )?;
        Ok(NextScreen::ItemListDetails(Box::new(parent), children))
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
            ItemDetailsCommand::Reload => ControlFlow::Break(Navigation::Replace(
                NextScreen::FetchItemDetails(self.id.clone()),
            )),
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
pub fn render_item_details(cx: TuiContext, item: Box<MediaItem>) -> Erased {
    let id = item.id.clone();
    let widget = ItemDetails::new(item, &cx);
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.item_details.clone(),
        DetailsMapper { id },
    );
    make_new_erased(cx, OuterWidget::<DetailsName, _>::new(widget))
}

struct ListMapper {
    id: String,
}

impl CommandMapper<ItemListDetailsCommand> for ListMapper {
    type A = jellyhaj_item_details_widget::item_list_details::ItemListDetailsAction;

    fn map(&self, command: ItemListDetailsCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            ItemListDetailsCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            ItemListDetailsCommand::Reload => ControlFlow::Break(Navigation::Replace(
                NextScreen::FetchItemListDetailsRef(self.id.clone()),
            )),
            ItemListDetailsCommand::Up => {
                ControlFlow::Continue(ItemListDetailsAction::Universal(ItemListDetailsCommom::Up))
            }
            ItemListDetailsCommand::Down => ControlFlow::Continue(
                ItemListDetailsAction::Universal(ItemListDetailsCommom::Down),
            ),
            ItemListDetailsCommand::ScrollUp => ControlFlow::Continue(
                ItemListDetailsAction::Universal(ItemListDetailsCommom::ScrollUp),
            ),
            ItemListDetailsCommand::ScrollDown => ControlFlow::Continue(
                ItemListDetailsAction::Universal(ItemListDetailsCommom::ScrollDown),
            ),
            ItemListDetailsCommand::NextTab => ControlFlow::Continue(ItemListDetailsAction::Next),
            ItemListDetailsCommand::PrevTab => ControlFlow::Continue(ItemListDetailsAction::Prev),
            ItemListDetailsCommand::Entry(entry_command) => ControlFlow::Continue(
                ItemListDetailsAction::Universal(ItemListDetailsCommom::Entry(entry_command)),
            ),
            ItemListDetailsCommand::RefreshParentItem => {
                ControlFlow::Break(Navigation::Push(NextScreen::RefreshItem(self.id.clone())))
            }
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
    cx: TuiContext,
    parent: Box<MediaItem>,
    children: Vec<MediaItem>,
) -> Erased {
    let id = parent.id.clone();
    let list_details = jellyhaj_item_details_widget::item_list_details::ItemListDetails::new(
        parent, children, &cx,
    );
    let tabbed = TabbedWidgets::new(list_details);
    let state = KeybindWidget::new(
        tabbed,
        cx.config.keybinds.item_list_details.clone(),
        ListMapper { id },
    );
    make_new_erased(cx, OuterWidget::<ListName, _>::new(state))
}
