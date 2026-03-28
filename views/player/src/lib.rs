use std::ops::ControlFlow;

use futures_util::future::try_join_all;
use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem, PlaybackInfo},
    playlist::GetPlaylistItemsQuery,
    shows::GetEpisodesQuery,
};
use jellyhaj_core::{
    CommandMapper,
    context::{DefaultTerminal, KeybindEvents, TuiContext},
    keybinds::MpvCommand,
    render::{NavigationResult, render_widget},
    state::{LoadPlay, Navigation, NextScreen},
};
use jellyhaj_keybinds_widget::KeybindState;
use jellyhaj_player_widget::{PlayerAction, PlayerWidget};
use jellyhaj_widgets_core::outer::{Named, OuterState};
use player_core::{Command, PlayItem};

use color_eyre::{
    Report, Result,
    eyre::{Context, eyre},
};
use tracing::warn;

struct Mapper;

impl CommandMapper<MpvCommand> for Mapper {
    type A = PlayerAction;

    fn map(&self, command: MpvCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            MpvCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            MpvCommand::Pause => ControlFlow::Continue(PlayerAction::TogglePause),
            MpvCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct Name;

impl Named for Name {
    const NAME: &str = "player";
}

pub fn render_play(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
    items: Vec<(MediaItem, PlaybackInfo)>,
    index: usize,
) -> impl Future<Output = NavigationResult> {
    cx.mpv_handle.send(Command::Minimized(false));
    cx.mpv_handle.send(Command::Fullscreen(true));
    cx.mpv_handle.send(Command::ReplacePlaylist {
        items: items.into_iter().map(PlayItem::from).collect(),
        first: index,
    });
    let state = OuterState::<Name, _, _, _, _>::new(KeybindState::new(
        PlayerWidget::new(cx.mpv_handle.clone()),
        cx.config.keybinds.play_mpv.clone(),
        Mapper,
    ));
    render_widget(term, events, cx, state)
}

pub fn render_fetch_play(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
    item: LoadPlay,
) -> impl Future<Output = NavigationResult> {
    let fut = fetch_items(cx.jellyfin.clone(), item);
    jellyhaj_fetch_view::make_fetch(term, events, cx, "Loading related items for playlist", fut)
}

async fn fetch_items(cx: JellyfinClient, item: LoadPlay) -> Result<NextScreen> {
    let (items, index) = match item {
        LoadPlay::Series { id } => (fetch_series(&cx, &id).await?, 0),
        LoadPlay::Season { series_id, id } => {
            let all = fetch_series(&cx, &series_id).await?;
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
            let all = fetch_series(&cx, &series_id).await?;

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
        LoadPlay::Movie(item) => (vec![*item], 0),
        LoadPlay::Music { id, album_id } => {
            let items = fetch_childs(&cx, &album_id).await?;
            let pos = item_position(&id, &items).unwrap_or(0);
            (items, pos)
        }
        LoadPlay::MusicAlbum { id } => (fetch_childs(&cx, &id).await?, 0),
    };

    let items = try_join_all(items.into_iter().map(|item| async {
        let info = cx
            .get_playback_info(&item.id)
            .await
            .context("getting playback info")?
            .deserialize()
            .await
            .context("parsing playback info")?;
        Ok::<_, Report>((item, info))
    }))
    .await?;
    if items.is_empty() {
        return Err(eyre!("Unable to play, item is empty"));
    }

    Ok(NextScreen::Play { items, index })
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
