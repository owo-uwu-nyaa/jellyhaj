use std::pin::Pin;

use color_eyre::{Result, eyre::Context};
use jellyfin::{JellyfinClient, JellyfinVec, items::GetItemsQuery, user_views::UserView};
use jellyhaj_core::{
    context::TuiContext,
    entries::EntryResultExt,
    keybinds::UserViewCommand,
    state::{Navigation, NextScreen},
};
use jellyhaj_entry_widget::{Entry, EntryAction, EntryData};
use jellyhaj_fetch_view::render_fetch_future;
use jellyhaj_item_grid::{DimensionsParameter, ItemGrid, ItemGridAction, ItemGridData};
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_render_widgets::{JellyhajWidget, TermExt};

async fn fetch_user_view(jellyfin: &JellyfinClient, view: UserView) -> Result<Navigation> {
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
    let title = view.name.clone();
    Ok(Navigation::Replace(NextScreen::UserView {
        view,
        items: ItemGridData {
            items: items.into_iter().map(EntryData::Item).collect(),
            title,
            current: 0,
        },
    }))
}

pub async fn render_fetch_user_view(
    cx: Pin<&mut TuiContext>,
    view: UserView,
) -> Result<Navigation> {
    let cx = cx.project();
    render_fetch_future(
        &format!("Loading user view {}", view.name),
        fetch_user_view(cx.jellyfin, view),
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}

#[derive(Debug)]
pub enum Pass {
    Reload,
    Quit,
}

pub async fn render_user_view(
    cx: Pin<&mut TuiContext>,
    library: ItemGridData<EntryData>,
    view: UserView,
) -> Result<Navigation> {
    let cx = cx.project();

    let mut widget = KeybindWidget::new(
        ItemGrid::new(
            library
                .items
                .into_iter()
                .map(|i| {
                    Entry::new(
                        i,
                        cx.jellyfin,
                        cx.cache,
                        cx.image_cache,
                        cx.image_picker,
                        cx.stats,
                        cx.config,
                    )
                })
                .collect(),
            library.current,
            library.title,
            DimensionsParameter {
                config: cx.config,
                font_size: cx.image_picker.font_size(),
            },
        ),
        &cx.config.help_prefixes,
        cx.config.keybinds.user_view.clone(),
        |command| match command {
            UserViewCommand::Quit => MappedCommand::Up(Pass::Quit),
            UserViewCommand::Reload => MappedCommand::Up(Pass::Reload),
            UserViewCommand::Prev => MappedCommand::Down(ItemGridAction::Left),
            UserViewCommand::Next => MappedCommand::Down(ItemGridAction::Right),
            UserViewCommand::Up => MappedCommand::Down(ItemGridAction::Up),
            UserViewCommand::Down => MappedCommand::Down(ItemGridAction::Down),
            UserViewCommand::Open => {
                MappedCommand::Down(ItemGridAction::CurrentInner(EntryAction::Open))
            }
            UserViewCommand::Play => {
                MappedCommand::Down(ItemGridAction::CurrentInner(EntryAction::Play))
            }
            UserViewCommand::OpenEpisode => {
                MappedCommand::Down(ItemGridAction::CurrentInner(EntryAction::OpenEpisode))
            }
            UserViewCommand::OpenSeason => {
                MappedCommand::Down(ItemGridAction::CurrentInner(EntryAction::OpenSeason))
            }
            UserViewCommand::OpenSeries => {
                MappedCommand::Down(ItemGridAction::CurrentInner(EntryAction::OpenSeries))
            }
            UserViewCommand::RefreshItem => {
                MappedCommand::Down(ItemGridAction::CurrentInner(EntryAction::Refresh))
            }
        },
    );

    Ok(loop {
        match cx
            .term
            .render(&mut widget, cx.events, cx.spawn.clone())
            .await?
        {
            CommandAction::Up(Pass::Reload) => {
                break Navigation::Replace(NextScreen::LoadUserView(view));
            }
            CommandAction::Up(Pass::Quit) => {
                break Navigation::PopContext;
            }
            CommandAction::Action(action) => {
                if let Some(next) = action.to_next_screen() {
                    break Navigation::Push {
                        current: NextScreen::UserView {
                            view,
                            items: widget.into_state(),
                        },
                        next,
                    };
                }
            }
            CommandAction::Exit => {
                break Navigation::Exit;
            }
        }
    })
}
