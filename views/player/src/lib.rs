use std::{collections::BTreeMap, ffi::CString, fmt::Debug, ops::ControlFlow};

use jellyfin::{
    JellyfinClient, JellyfinVec,
    items::{GetItemsQuery, MediaItem},
    playlist::GetPlaylistItemsQuery,
    shows::GetEpisodesQuery,
};
use jellyhaj_context::TuiContext;
use jellyhaj_core::{
    CommandMapper,
    keybinds::MpvCommand,
    state::{LoadPlay, Navigation, NextScreen},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_player_widget::{PlayerAction, PlayerWidget};
use jellyhaj_widgets_core::{
    outer::{Named, OuterWidget},
    valuable::{Fields, NamedField, NamedValues, StructDef, Structable, Valuable, Value, Visit},
};
use mpv_async::nodes::MpvOwnedNode;
use player_core::{Command, PlayerHandle};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt, eyre},
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
            MpvCommand::Forward => ControlFlow::Continue(PlayerAction::Forward),
            MpvCommand::Backward => ControlFlow::Continue(PlayerAction::Backward),
            MpvCommand::Next => ControlFlow::Continue(PlayerAction::Next),
            MpvCommand::Prev => ControlFlow::Continue(PlayerAction::Prev),
            MpvCommand::Inspect => ControlFlow::Break(Navigation::Push(NextScreen::InspectPlayer)),
        }
    }
}

struct Name;

impl Named for Name {
    const NAME: &str = "player";
}

pub fn render_play(cx: TuiContext, items: Vec<MediaItem>, index: usize) -> Erased {
    cx.mpv_handle.send(Command::Minimized(false));
    cx.mpv_handle.send(Command::Fullscreen(true));
    cx.mpv_handle.send(Command::ReplacePlaylist {
        items,
        first: index,
    });
    let widget = OuterWidget::<Name, _>::new(KeybindWidget::new(
        PlayerWidget::new(cx.mpv_handle.clone()),
        cx.config.keybinds.play_mpv.clone(),
        Mapper,
    ));
    make_new_erased(cx, widget)
}

pub fn render_fetch_play(cx: TuiContext, item: LoadPlay) -> Erased {
    let fut = fetch_items(cx.jellyfin.clone(), item);
    jellyhaj_fetch_view::make_fetch(cx, "Loading related items for playlist", fut)
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
                    .deserialize()?;
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
                .context("deserializing playlist items")
            })
            .await?;
            (items, 0)
        }
        LoadPlay::Movie(item) | LoadPlay::Audio(item) => (vec![*item], 0),
        LoadPlay::Music { id, album_id } => {
            let items = fetch_childs(&cx, &album_id).await?;
            let pos = item_position(&id, &items).unwrap_or(0);
            (items, pos)
        }
        LoadPlay::MusicAlbum { id } => (fetch_childs(&cx, &id).await?, 0),
    };

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
            include_item_types: "Movie,Episode,Music,Audio".into(),
            ..Default::default()
        })
        .await
        .context("fetching media items")?
        .deserialize()
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
        .context("deserializing media items")
    })
    .await?;
    Ok(res)
}

struct Properties {
    inner: BTreeMap<String, MpvOwnedNode>,
}

impl Debug for Properties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut d = f.debug_struct("Properties");
        for (key, val) in &self.inner {
            d.field(key, val);
        }
        d.finish()
    }
}

impl Valuable for Properties {
    fn as_value(&self) -> Value<'_> {
        Value::Structable(self)
    }

    fn visit(&self, visit: &mut dyn Visit) {
        for (key, val) in &self.inner {
            visit.visit_named_fields(&NamedValues::new(
                &[NamedField::new(key)],
                &[val.as_value()],
            ));
        }
    }
}

impl Structable for Properties {
    fn definition(&self) -> StructDef<'_> {
        StructDef::new_dynamic("Properties", Fields::Named(&[]))
    }
}

async fn fetch_properties(handle: PlayerHandle) -> Result<NextScreen> {
    async fn fetch_property(name: CString, handle: &PlayerHandle) -> Result<MpvOwnedNode> {
        let (sender, recv) = tokio::sync::oneshot::channel();
        handle.send(Command::GetProperty { name, sender });
        recv.await.context("player has been closed")?
    }
    let properties = fetch_property(c"property-list".to_owned(), &handle).await?;
    let mut properties_full = BTreeMap::new();
    for name in properties
        .differentiate()
        .array()
        .ok_or_eyre("property list is not a list")?
    {
        let name = name
            .differentiate()
            .string()
            .ok_or_eyre("property name is not a string")?;
        let Ok(val) = fetch_property(name.to_owned(), &handle).await else {
            warn!("unavailable property {name:?}");
            continue;
        };
        let name = name
            .to_str()
            .context("mpv property is not utf-8")?
            .to_string();
        properties_full.insert(name, val);
    }

    Ok(NextScreen::InspectValuable(Box::new(Properties {
        inner: properties_full,
    })))
}

pub fn make_fetch_properties(cx: TuiContext) -> Erased {
    let fut = fetch_properties(cx.mpv_handle.clone());
    jellyhaj_fetch_view::make_fetch(cx, "Collecting player properties", fut)
}
