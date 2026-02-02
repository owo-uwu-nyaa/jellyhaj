use jellyfin::items::{ItemType, MediaItem};
use jellyhaj_entry_widget::{EntryData, EntryResult};

use crate::state::{LoadPlay, NextScreen};

pub trait EntryExt {
    fn item_id(&self) -> Option<&str>;
}

pub trait EntryResultExt {
    fn to_next_screen(self) -> Option<NextScreen>;
}

impl EntryResultExt for EntryResult {
    fn to_next_screen(self) -> Option<NextScreen> {
        match self {
            EntryResult::Activate(EntryData::Item(item)) => Some(play_item(item)),
            EntryResult::Activate(EntryData::View(view)) => Some(NextScreen::LoadUserView(view)),
            EntryResult::Play(EntryData::Item(item)) => Some(play_item(item)),
            EntryResult::Open(EntryData::Item(item)) => Some(open_item(item)),
            EntryResult::Open(EntryData::View(view)) => Some(NextScreen::LoadUserView(view)),
            EntryResult::OpenSeries(EntryData::Item(item)) => item_series(item),
            EntryResult::OpenSeason(EntryData::Item(item)) => item_season(item),
            EntryResult::OpenEpisode(EntryData::Item(item)) => Some(item_episode(item)),
            EntryResult::Refresh(EntryData::Item(item)) => Some(NextScreen::RefreshItem(item.id)),
            EntryResult::Refresh(EntryData::View(view)) => Some(NextScreen::RefreshItem(view.id)),
            EntryResult::Play(EntryData::View(_))
            | EntryResult::OpenSeries(EntryData::View(_))
            | EntryResult::OpenSeason(EntryData::View(_))
            | EntryResult::OpenEpisode(EntryData::View(_)) => None,
        }
    }
}

impl EntryExt for EntryData {
    fn item_id(&self) -> Option<&str> {
        match self {
            EntryData::Item(media_item) => Some(media_item.id.as_str()),
            EntryData::View(_) => None,
        }
    }
}
pub fn play_item(item: MediaItem) -> NextScreen {
    NextScreen::LoadPlayItem(match item {
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
        } => return NextScreen::UnsupportedItem,
    })
}

fn open_item(item: MediaItem) -> NextScreen {
    match item {
        v @ MediaItem {
            item_type: ItemType::Movie | ItemType::Music { .. } | ItemType::Episode { .. },
            ..
        } => NextScreen::ItemDetails(v),
        v @ MediaItem {
            item_type:
                ItemType::Playlist
                | ItemType::Folder
                | ItemType::Series
                | ItemType::MusicAlbum
                | ItemType::CollectionFolder
                | ItemType::Season { .. },
            ..
        } => NextScreen::FetchItemListDetails(v),
        MediaItem {
            item_type: ItemType::Unknown,
            ..
        } => NextScreen::UnsupportedItem,
    }
}
fn item_episode(item: MediaItem) -> NextScreen {
    match item {
        v @ MediaItem {
            item_type: ItemType::Movie | ItemType::Music { .. } | ItemType::Episode { .. },
            ..
        } => NextScreen::ItemDetails(v),
        i @ MediaItem {
            item_type:
                ItemType::Playlist | ItemType::MusicAlbum | ItemType::Series | ItemType::Season { .. },
            ..
        } => NextScreen::ItemDetails(i),
        MediaItem {
            item_type: ItemType::Unknown | ItemType::Folder | ItemType::CollectionFolder,
            ..
        } => NextScreen::UnsupportedItem,
    }
}

pub fn item_season(item: MediaItem) -> Option<NextScreen> {
    match item {
        MediaItem {
            item_type:
                ItemType::Episode {
                    season_id: Some(id),
                    ..
                },
            ..
        } => Some(NextScreen::FetchItemListDetailsRef(id)),
        i @ MediaItem {
            item_type: ItemType::Season { .. },
            ..
        } => Some(NextScreen::FetchItemListDetails(i)),
        i @ MediaItem {
            item_type: ItemType::Series,
            ..
        } => Some(NextScreen::FetchItemListDetails(i)),
        MediaItem {
            item_type: ItemType::Music { album_id, .. },
            ..
        } => Some(NextScreen::FetchItemListDetailsRef(album_id)),
        i @ MediaItem {
            item_type: ItemType::MusicAlbum,
            ..
        } => Some(NextScreen::FetchItemListDetails(i)),
        MediaItem {
            item_type: ItemType::Unknown | ItemType::CollectionFolder,
            ..
        } => Some(NextScreen::UnsupportedItem),
        _ => None,
    }
}

fn item_series(item: MediaItem) -> Option<NextScreen> {
    match item {
        MediaItem {
            item_type: ItemType::Episode { series_id, .. } | ItemType::Season { series_id, .. },
            ..
        } => Some(NextScreen::FetchItemListDetailsRef(series_id.clone())),
        i @ MediaItem {
            item_type: ItemType::Series,
            ..
        } => Some(NextScreen::FetchItemListDetails(i)),
        _ => None,
    }
}
