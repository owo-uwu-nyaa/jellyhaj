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
            (EntryData::Item(item), EntryCommand::Activate | EntryCommand::Play) => {
                play_item(item.clone())?
            }
            (EntryData::View(view), EntryCommand::Activate | EntryCommand::Open) => {
                NextScreen::LoadUserView(view.clone())
            }
            (
                EntryData::Item(MediaItem { id, .. }) | EntryData::View(UserView { id, .. }),
                EntryCommand::RefreshItem,
            ) => NextScreen::RefreshItem(id.clone()),
            (EntryData::Item(item), EntryCommand::Open) => open_item(item)?,
            (EntryData::Item(item), EntryCommand::OpenSeries) => item_series(item)?,
            (EntryData::Item(item), EntryCommand::OpenSeason) => item_season(item)?,
            (EntryData::Item(item), EntryCommand::OpenEpisode) => item_episode(item)?,
            (EntryData::Item(item), EntryCommand::SetWatched) => {
                let jellyfin = JellyfinClient::get_ref(cx).clone();
                let id = item.id.clone();
                Spawner::get_ref(cx).spawn_res(
                    async move { jellyfin.set_played(&id).await },
                    info_span!("set_watched"),
                    "set_watched",
                );
                return None;
            }
            (EntryData::Item(item), EntryCommand::UnsetWatched) => {
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
                EntryData::View(_),
                EntryCommand::Play
                | EntryCommand::OpenSeries
                | EntryCommand::OpenSeason
                | EntryCommand::OpenEpisode
                | EntryCommand::SetWatched
                | EntryCommand::UnsetWatched,
            ) => return None,
        };
        Some(Navigation::Push(Box::new(next)))
    }
}

pub fn play_item(item: MediaItem) -> Option<NextScreen> {
    Some(NextScreen::FetchPlay(match item {
        v @ MediaItem {
            item_type: ItemType::Movie,
            ..
        } => LoadPlay::Movie(v),
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
            item_type: ItemType::Unknown | ItemType::CollectionFolder,
            ..
        } => return None,
    }))
}

fn open_item(item: &MediaItem) -> Option<NextScreen> {
    Some(match item {
        v @ MediaItem {
            item_type: ItemType::Movie | ItemType::Music { .. } | ItemType::Episode { .. },
            ..
        } => NextScreen::ItemDetails(v.clone()),
        v @ MediaItem {
            item_type:
                ItemType::Playlist
                | ItemType::Folder
                | ItemType::Series
                | ItemType::MusicAlbum
                | ItemType::CollectionFolder
                | ItemType::Season { .. },
            ..
        } => NextScreen::FetchItemListDetails(v.clone()),
        MediaItem {
            item_type: ItemType::Unknown,
            ..
        } => return None,
    })
}
fn item_episode(item: &MediaItem) -> Option<NextScreen> {
    Some(match item {
        v @ MediaItem {
            item_type: ItemType::Movie | ItemType::Music { .. } | ItemType::Episode { .. },
            ..
        } => NextScreen::ItemDetails(v.clone()),
        i @ MediaItem {
            item_type:
                ItemType::Playlist | ItemType::MusicAlbum | ItemType::Series | ItemType::Season { .. },
            ..
        } => NextScreen::ItemDetails(i.clone()),
        MediaItem {
            item_type: ItemType::Unknown | ItemType::Folder | ItemType::CollectionFolder,
            ..
        } => return None,
    })
}

pub fn item_season(item: &MediaItem) -> Option<NextScreen> {
    Some(match item {
        MediaItem {
            item_type:
                ItemType::Episode {
                    season_id: Some(id),
                    ..
                },
            ..
        } => NextScreen::FetchItemListDetailsRef(id.clone()),
        i @ MediaItem {
            item_type: ItemType::Season { .. },
            ..
        } => NextScreen::FetchItemListDetails(i.clone()),
        i @ MediaItem {
            item_type: ItemType::Series,
            ..
        } => NextScreen::FetchItemListDetails(i.clone()),
        MediaItem {
            item_type: ItemType::Music { album_id, .. },
            ..
        } => NextScreen::FetchItemListDetailsRef(album_id.clone()),
        i @ MediaItem {
            item_type: ItemType::MusicAlbum,
            ..
        } => NextScreen::FetchItemListDetails(i.clone()),
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
        } => Some(NextScreen::FetchItemListDetails(i.clone())),
        _ => None,
    }
}
