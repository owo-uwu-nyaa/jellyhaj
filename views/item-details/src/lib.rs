use std::pin::Pin;

use color_eyre::{Result, eyre::Context};
use jellyfin::items::MediaItem;
use jellyhaj_core::{
    context::TuiContext,
    entries::EntryResultExt,
    keybinds::{ItemDetailsCommand, ItemListDetailsCommand},
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryData, EntryResult};
use jellyhaj_fetch_view::{
    fetch_all_children, fetch_child_of_type, fetch_item, render_fetch_future,
};
use jellyhaj_item_details_widget::{
    DisplayAction, DisplayListAction, ItemDisplay, ItemListDisplay,
};
use jellyhaj_item_list::{
    DimensionsParameter, ItemList, ItemListAction::CurrentInner, ItemListData,
};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_render_widgets::{JellyhajWidget, TermExt};
use tokio::try_join;
use tracing::instrument;

#[instrument(skip_all)]
pub async fn render_fetch_episode(cx: Pin<&mut TuiContext>, parent: String) -> Result<Navigation> {
    let cx = cx.project();
    render_fetch_future(
        "Fetching single item",
        async {
            let item = fetch_child_of_type(cx.jellyfin, "Episode, Movie, Music", &parent)
                .await
                .context("fetching episode")?;
            Ok(Navigation::Replace(NextScreen::ItemDetails(item)))
        },
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}

#[instrument(skip_all)]
pub async fn render_fetch_item_list(
    cx: Pin<&mut TuiContext>,
    parent: MediaItem,
) -> Result<Navigation> {
    let cx = cx.project();
    render_fetch_future(
        &format!("Loading {}", &parent.name),
        async {
            let items = fetch_all_children(cx.jellyfin, &parent.id).await?;
            let name = parent.name.clone();
            Ok(Navigation::Replace(NextScreen::ItemListDetails(
                parent,
                ItemListData::new(items.into_iter().map(EntryData::Item), name),
            )))
        },
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}

#[instrument(skip_all)]
pub async fn render_fetch_item_list_ref(
    cx: Pin<&mut TuiContext>,
    parent: String,
) -> Result<Navigation> {
    let cx = cx.project();
    render_fetch_future(
        "Loading item list",
        async {
            let (parent, children) = try_join!(
                fetch_item(cx.jellyfin, &parent),
                fetch_all_children(cx.jellyfin, &parent)
            )?;
            let name = parent.name.clone();
            Ok(Navigation::Replace(NextScreen::ItemListDetails(
                parent,
                ItemListData::new(children.into_iter().map(EntryData::Item), name),
            )))
        },
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}

#[derive(Debug)]
struct ItemDetailsQuit;

#[instrument(skip_all)]
pub async fn render_item_details(cx: Pin<&mut TuiContext>, item: MediaItem) -> Result<Navigation> {
    let cx = cx.project();
    let mut widget = KeybindWidget::new(
        ItemDisplay::new(
            item,
            cx.jellyfin,
            cx.cache,
            cx.image_cache,
            cx.image_picker,
            cx.stats,
            cx.config,
        ),
        &cx.config.help_prefixes,
        cx.config.keybinds.item_details.clone(),
        |command| match command {
            ItemDetailsCommand::Quit => MappedCommand::Up(ItemDetailsQuit),
            ItemDetailsCommand::Up => MappedCommand::Down(DisplayAction::Up),
            ItemDetailsCommand::Down => MappedCommand::Down(DisplayAction::Down),
            ItemDetailsCommand::Play => {
                MappedCommand::Down(DisplayAction::Inner(EntryAction::Play))
            }
            ItemDetailsCommand::Reload => {
                MappedCommand::Down(DisplayAction::Inner(EntryAction::OpenEpisode))
            }
            ItemDetailsCommand::RefreshItem => {
                MappedCommand::Down(DisplayAction::Inner(EntryAction::Refresh))
            }
        },
    );
    Ok(loop {
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Action(v @ EntryResult::Refresh(_)) => {
                if let Some(next) = v.to_next_screen() {
                    break Navigation::Replace(next);
                }
            }
            CommandAction::Action(v) => {
                if let Some(next) = v.to_next_screen() {
                    break Navigation::Push {
                        current: NextScreen::ItemDetails(widget.into_state()),
                        next,
                    };
                }
            }
            CommandAction::Up(ItemDetailsQuit) => break Navigation::PopContext,
            CommandAction::Exit => break Navigation::Exit,
        }
    })
}

#[derive(Debug)]
enum ItemDetailsListPass {
    Quit,
    Reload,
    RefreshParent,
}

#[instrument(skip_all)]
pub async fn render_item_list_details(
    cx: Pin<&mut TuiContext>,
    parent: MediaItem,
    children: ItemListData<EntryData>,
) -> Result<Navigation> {
    let cx = cx.project();
    let mut widget = KeybindWidget::new(
        ItemListDisplay::new(
            parent,
            ItemList::new(
                children.items.into_iter().map(|i| {
                    Entry::new(
                        i,
                        cx.jellyfin,
                        cx.cache,
                        cx.image_cache,
                        cx.image_picker,
                        cx.stats,
                        cx.config,
                    )
                }),
                children.current,
                children.title,
                DimensionsParameter {
                    config: cx.config,
                    font_size: cx.image_picker.font_size(),
                },
            ),
        ),
        &cx.config.help_prefixes,
        cx.config.keybinds.item_list_details.clone(),
        |command| match command {
            ItemListDetailsCommand::Quit => MappedCommand::Up(ItemDetailsListPass::Quit),
            ItemListDetailsCommand::Reload => MappedCommand::Up(ItemDetailsListPass::Reload),
            ItemListDetailsCommand::Up => MappedCommand::Down(DisplayListAction::Up),
            ItemListDetailsCommand::Down => MappedCommand::Down(DisplayListAction::Down),
            ItemListDetailsCommand::Left => MappedCommand::Down(DisplayListAction::Left),
            ItemListDetailsCommand::Right => MappedCommand::Down(DisplayListAction::Right),
            ItemListDetailsCommand::Play => {
                MappedCommand::Down(DisplayListAction::Inner(CurrentInner(EntryAction::Play)))
            }
            ItemListDetailsCommand::Open => {
                MappedCommand::Down(DisplayListAction::Inner(CurrentInner(EntryAction::Open)))
            }
            ItemListDetailsCommand::OpenEpisode => MappedCommand::Down(DisplayListAction::Inner(
                CurrentInner(EntryAction::OpenEpisode),
            )),
            ItemListDetailsCommand::OpenSeason => MappedCommand::Down(DisplayListAction::Inner(
                CurrentInner(EntryAction::OpenSeason),
            )),
            ItemListDetailsCommand::OpenSeries => MappedCommand::Down(DisplayListAction::Inner(
                CurrentInner(EntryAction::OpenSeries),
            )),
            ItemListDetailsCommand::RefreshCurrentItem => {
                MappedCommand::Down(DisplayListAction::Inner(CurrentInner(EntryAction::Refresh)))
            }
            ItemListDetailsCommand::RefreshParentItem => {
                MappedCommand::Up(ItemDetailsListPass::RefreshParent)
            }
        },
    );
    Ok(loop {
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Action(inner) => {
                if let Some(next) = inner.to_next_screen() {
                    let state = widget.into_state();
                    break Navigation::Push {
                        current: NextScreen::ItemListDetails(state.item, state.children),
                        next,
                    };
                }
            }
            CommandAction::Up(ItemDetailsListPass::Quit) => {
                break Navigation::PopContext;
            }
            CommandAction::Up(ItemDetailsListPass::Reload) => {
                break Navigation::Replace(NextScreen::FetchItemListDetailsRef(
                    widget.into_state().item.id,
                ));
            }
            CommandAction::Up(ItemDetailsListPass::RefreshParent) => {
                let state = widget.into_state();
                let id = state.item.id.clone();
                break Navigation::Push {
                    current: NextScreen::ItemListDetails(state.item, state.children),
                    next: NextScreen::RefreshItem(id),
                };
            }
            CommandAction::Exit => break Navigation::Exit,
        }
    })
}
