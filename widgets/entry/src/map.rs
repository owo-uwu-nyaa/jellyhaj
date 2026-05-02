use jellyfin::{
    JellyfinClient,
    items::{ItemType, MediaItem},
    user_views::UserView,
};
use jellyhaj_core::{
    context::Spawner,
    keybinds::EntryCommand,
    state::{LoadPlay, Navigation, NextScreen},
};
use jellyhaj_widgets_core::{ContextRef, GetFromContext};
use tracing::info_span;

use crate::EntryData;

impl EntryData {
    pub fn apply_command(
        &self,
        command: EntryCommand,
        cx: &(impl ContextRef<Spawner> + ContextRef<JellyfinClient>),
    ) -> Option<Navigation> {
        let next: NextScreen = match (self, command) {
            (Self::Item(item), EntryCommand::Activate | EntryCommand::Play) => {
                play_item(item.clone())?
            }
            (Self::View(view), EntryCommand::Activate | EntryCommand::Open) => {
                NextScreen::LoadUserView(Box::new(view.clone()))
            }
            (
                Self::Item(MediaItem { id, .. }) | Self::View(UserView { id, .. }),
                EntryCommand::RefreshItem,
            ) => NextScreen::RefreshItem(id.clone()),
            (Self::Item(item), EntryCommand::Open) => open_item(item)?,
            (Self::Item(item), EntryCommand::OpenSeries) => item_series(item)?,
            (Self::Item(item), EntryCommand::OpenSeason) => item_season(item)?,
            (Self::Item(item), EntryCommand::OpenEpisode) => item_episode(item)?,
            (Self::Item(item), EntryCommand::SetWatched) => {
                let jellyfin = JellyfinClient::get_ref(cx).clone();
                let id = item.id.clone();
                Spawner::get_ref(cx).spawn_res(
                    async move { jellyfin.set_played(&id).await },
                    info_span!("set_watched"),
                    "set_watched",
                );
                return None;
            }
            (Self::Item(item), EntryCommand::UnsetWatched) => {
                let jellyfin = JellyfinClient::get_ref(cx).clone();
                let id = item.id.clone();
                Spawner::get_ref(cx).spawn_res(
                    async move { jellyfin.set_unplayed(&id).await },
                    info_span!("unset_watched"),
                    "unset_watched",
                );
                return None;
            }
            (
                Self::View(_),
                EntryCommand::Play
                | EntryCommand::OpenSeries
                | EntryCommand::OpenSeason
                | EntryCommand::OpenEpisode
                | EntryCommand::SetWatched
                | EntryCommand::UnsetWatched,
            ) => return None,
        };
        Some(Navigation::Push(next))
    }
}

#[must_use]
pub fn play_item(item: MediaItem) -> Option<NextScreen> {
    Some(NextScreen::FetchPlay(match item {
        v @ MediaItem {
            item_type: ItemType::Movie,
            ..
        } => LoadPlay::Movie(Box::new(v)),
        MediaItem {
            id,
            item_type: ItemType::Playlist | ItemType::Folder,
            ..
        } => LoadPlay::Playlist { id },
        MediaItem {
            id,
            item_type: ItemType::Series,
            ..
        } => LoadPlay::Series { id },
        MediaItem {
            id,
            item_type: ItemType::Season { series_id, .. },
            ..
        } => LoadPlay::Season { series_id, id },
        MediaItem {
            id,
            item_type: ItemType::Episode { series_id, .. },
            ..
        } => LoadPlay::Episode { series_id, id },
        MediaItem {
            id,
            item_type: ItemType::Music { album_id, .. },
            ..
        } => LoadPlay::Music { id, album_id },
        MediaItem {
            id,
            item_type: ItemType::MusicAlbum,
            ..
        } => LoadPlay::MusicAlbum { id },
        MediaItem {
            item_type: ItemType::Unknown { item_type: _ } | ItemType::CollectionFolder,
            ..
        } => return None,
    }))
}

fn open_item(item: &MediaItem) -> Option<NextScreen> {
    Some(match item {
        v @ MediaItem {
            item_type: ItemType::Movie | ItemType::Music { .. } | ItemType::Episode { .. },
            ..
        } => NextScreen::ItemDetails(Box::new(v.clone())),
        v @ MediaItem {
            item_type:
                ItemType::Playlist
                | ItemType::Folder
                | ItemType::Series
                | ItemType::MusicAlbum
                | ItemType::CollectionFolder
                | ItemType::Season { .. },
            ..
        } => NextScreen::FetchItemListDetails(Box::new(v.clone())),
        MediaItem {
            item_type: ItemType::Unknown { item_type: _ },
            ..
        } => return None,
    })
}
fn item_episode(item: &MediaItem) -> Option<NextScreen> {
    Some(match item {
        v @ MediaItem {
            item_type: ItemType::Movie | ItemType::Music { .. } | ItemType::Episode { .. },
            ..
        } => NextScreen::ItemDetails(Box::new(v.clone())),
        i @ MediaItem {
            item_type:
                ItemType::Playlist | ItemType::MusicAlbum | ItemType::Series | ItemType::Season { .. },
            ..
        } => NextScreen::ItemDetails(Box::new(i.clone())),
        MediaItem {
            item_type:
                ItemType::Unknown { item_type: _ } | ItemType::Folder | ItemType::CollectionFolder,
            ..
        } => return None,
    })
}

#[must_use]
pub fn item_season(item: &MediaItem) -> Option<NextScreen> {
    Some(match item {
        MediaItem {
            item_type:
                ItemType::Episode {
                    season_id: Some(id),
                    ..
                }
                | ItemType::Music { album_id: id, .. },
            ..
        } => NextScreen::FetchItemListDetailsRef(id.clone()),
        i @ MediaItem {
            item_type: ItemType::Season { .. } | ItemType::Series | ItemType::MusicAlbum,
            ..
        } => NextScreen::FetchItemListDetails(Box::new(i.clone())),
        _ => return None,
    })
}

fn item_series(item: &MediaItem) -> Option<NextScreen> {
    match item {
        MediaItem {
            item_type: ItemType::Episode { series_id, .. } | ItemType::Season { series_id, .. },
            ..
        } => Some(NextScreen::FetchItemListDetailsRef(series_id.clone())),
        i @ MediaItem {
            item_type: ItemType::Series,
            ..
        } => Some(NextScreen::FetchItemListDetails(Box::new(i.clone()))),
        _ => None,
    }
}
