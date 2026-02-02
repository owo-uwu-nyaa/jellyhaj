use std::{convert::Infallible, pin::Pin};

use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
    playlist::GetPlaylistItemsQuery,
    shows::GetEpisodesQuery,
};
use jellyhaj_core::{
    context::TuiContext,
    keybinds::MpvCommand,
    state::{LoadPlay, Navigation, NextScreen},
};
use jellyhaj_fetch_view::render_fetch_future;
use jellyhaj_keybinds_widget::{KeybindWidget, MappedCommand};
use jellyhaj_player_widget::{PlayerAction, PlayerQuit, PlayerWidget};
use jellyhaj_render_widgets::TermExt;
use player_core::{Command, PlayerHandle};

use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use tracing::warn;

struct MinimizeGuard<'i> {
    handle: &'i PlayerHandle,
}

impl Drop for MinimizeGuard<'_> {
    fn drop(&mut self) {
        self.handle.send(Command::Stop);
    }
}

pub async fn render_play(
    cx: Pin<&mut TuiContext>,
    items: Vec<MediaItem>,
    index: usize,
) -> Result<Navigation> {
    if items.is_empty() {
        return Ok(Navigation::Replace(NextScreen::Error(eyre!(
            "Unable to play, item is empty"
        ))));
    }
    let cx = cx.project();
    cx.mpv_handle.send(Command::Minimized(false));
    cx.mpv_handle.send(Command::Fullscreen(true));
    cx.mpv_handle.send(Command::ReplacePlaylist {
        items,
        first: index,
    });
    let minimize = MinimizeGuard {
        handle: cx.mpv_handle,
    };
    let mut widget = KeybindWidget::new(
        PlayerWidget::new(cx.mpv_handle.clone()),
        &cx.config.help_prefixes,
        cx.config.keybinds.play_mpv.clone(),
        |command| -> MappedCommand<Infallible, PlayerAction> {
            match command {
                MpvCommand::Quit => MappedCommand::Down(PlayerAction::Quit),
                MpvCommand::Pause => MappedCommand::Down(PlayerAction::TogglePause),
            }
        },
    );
    let res = match cx
        .term
        .render(&mut widget, cx.events, cx.spawn.clone())
        .await?
    {
        jellyhaj_keybinds_widget::CommandAction::Action(PlayerQuit) => Ok(Navigation::PopContext),
        jellyhaj_keybinds_widget::CommandAction::Exit => Ok(Navigation::Exit),
    };
    drop(minimize);
    res
}

pub async fn render_fetch_play(cx: Pin<&mut TuiContext>, item: LoadPlay) -> Result<Navigation> {
    let cx = cx.project();
    render_fetch_future(
        "Loading related items for playlist",
        fetch_items(cx.jellyfin, item),
        cx.events,
        cx.config.keybinds.fetch.clone(),
        cx.term,
        &cx.config.help_prefixes,
        cx.spawn.clone(),
    )
    .await
}

async fn fetch_items(cx: &JellyfinClient, item: LoadPlay) -> Result<Navigation> {
    let (items, index) = match item {
        LoadPlay::Series { id } => (fetch_series(cx, &id).await?, 0),
        LoadPlay::Season { series_id, id } => {
            let all = fetch_series(cx, &series_id).await?;
            let user_id = cx.get_auth().user.id.as_str();
            let season_items = cx
                .get_episodes(
                    &series_id,
                    &GetEpisodesQuery {
                        user_id: user_id.into(),
                        is_missing: false.into(),
                        start_index: 0.into(),
                        limit: 1.into(),
                        season_id: id.as_str().into(),
                        enable_images: false.into(),
                        enable_user_data: false.into(),
                        ..Default::default()
                    },
                )
                .await
                .context("fetching media items")?
                .deserialize()
                .await
                .context("deserializing media items")?
                .items;
            if let Some(first) = season_items.first() {
                if let Some(p) = item_position(&first.id, &all) {
                    (all, p)
                } else {
                    (season_items, 0)
                }
            } else {
                warn!("no items found for season");
                (all, 0)
            }
        }
        LoadPlay::Episode { series_id, id } => {
            let all = fetch_series(cx, &series_id).await?;

            if let Some(position) = item_position(&id, &all) {
                (all, position)
            } else {
                let item = cx
                    .get_item(&id, Some(&cx.get_auth().user.id))
                    .await?
                    .deserialize()
                    .await?;
                (vec![item], 0)
            }
        }
        LoadPlay::Playlist { id } => {
            let user_id = cx.get_auth().user.id.as_str();
            let items = JellyfinVec::collect(async |start| {
                cx.get_playlist_items(
                    &id,
                    &GetPlaylistItemsQuery {
                        user_id: user_id.into(),
                        start_index: start.into(),
                        limit: 100.into(),
                        enable_images: Some(true),
                        image_type_limit: 1.into(),
                        enable_image_types: "Primary, Backdrop, Thumb".into(),
                        enable_user_data: true.into(),
                    },
                )
                .await
                .context("fetching playlist items")?
                .deserialize()
                .await
                .context("deserializing playlist items")
            })
            .await?;
            (items, 0)
        }
        LoadPlay::Movie(item) => (vec![item], 0),
        LoadPlay::Music { id, album_id } => {
            let items = fetch_childs(cx, &album_id).await?;
            let pos = item_position(&id, &items).unwrap_or(0);
            (items, pos)
        }
        LoadPlay::MusicAlbum { id } => (fetch_childs(cx, &id).await?, 0),
    };
    Ok(Navigation::Replace(NextScreen::Play { items, index }))
}

fn item_position(id: &str, items: &[MediaItem]) -> Option<usize> {
    for (index, item) in items.iter().enumerate() {
        if item.id == id {
            return Some(index);
        }
    }
    warn!("no such item found");
    None
}

async fn fetch_childs(cx: &JellyfinClient, parent_id: &str) -> Result<Vec<MediaItem>> {
    let user_id = cx.get_auth().user.id.as_str();
    let res = JellyfinVec::collect(async |start| {
        cx.get_items(&GetItemsQuery {
            user_id: user_id.into(),
            start_index: start.into(),
            limit: 100.into(),
            parent_id: parent_id.into(),
            enable_images: Some(true),
            image_type_limit: 1.into(),
            enable_image_types: "Primary, Backdrop, Thumb".into(),
            enable_user_data: true.into(),
            sort_by: "ParentIndexNumber,IndexNumber,SortName".into(),
            recursive: true.into(),
            ..Default::default()
        })
        .await
        .context("fetching media items")?
        .deserialize()
        .await
        .context("deserializing media items")
    })
    .await?;
    Ok(res)
}

async fn fetch_series(cx: &JellyfinClient, series_id: &str) -> Result<Vec<MediaItem>> {
    let user_id = cx.get_auth().user.id.as_str();
    let res = JellyfinVec::collect(async |start| {
        cx.get_episodes(
            series_id,
            &GetEpisodesQuery {
                user_id: user_id.into(),
                is_missing: false.into(),
                start_index: start.into(),
                limit: 100.into(),
                enable_images: Some(true),
                image_type_limit: 1.into(),
                enable_image_types: "Primary, Backdrop, Thumb".into(),
                enable_user_data: true.into(),
                ..Default::default()
            },
        )
        .await
        .context("fetching media items")?
        .deserialize()
        .await
        .context("deserializing media items")
    })
    .await?;
    Ok(res)
}
