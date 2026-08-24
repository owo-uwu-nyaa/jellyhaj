use std::collections::HashMap;
use std::fmt::Display;

use crate::Authed;
use crate::request::{NoQuery, RequestBuilderExt};
use crate::user::MediaSource;
use crate::{JellyfinClient, JellyfinVec, Result, connect::JsonResponse};
use color_eyre::eyre::{Context, eyre};
use http::Uri;
use serde::Deserialize;
use serde::Serialize;
use strum::IntoStaticStr;
use tracing::{debug, instrument};

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserIdQuery<'a> {
    pub user_id: Option<&'a str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefreshItemQuery {
    pub recursive: bool,
    pub metadata_refresh_mode: RefreshMode,
    pub image_refresh_mode: RefreshMode,
    pub replace_all_metadata: bool,
    pub replace_all_images: bool,
    pub regenerate_trickplay: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetVideoQuery<'s> {
    #[serde(rename = "static")]
    use_original: &'s str,
    media_source_id: &'s str,
    play_session_id: &'s str,
    api_key: &'s str,
    device_id: &'s str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
struct GetAudioQuery<'s> {
    #[serde(rename = "static")]
    use_original: &'s str,
    play_session_id: &'s str,
    api_key: &'s str,
    device_id: &'s str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
struct DownloadQuery<'s> {
    api_key: &'s str,
}

impl Default for RefreshItemQuery {
    fn default() -> Self {
        Self {
            recursive: true,
            metadata_refresh_mode: RefreshMode::Default,
            image_refresh_mode: RefreshMode::Default,
            replace_all_metadata: false,
            replace_all_images: false,
            regenerate_trickplay: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Eq)]
pub enum RefreshMode {
    None,
    ValidationOnly,
    Default,
    FullRefresh,
}

macro_rules! join_concat {
    () => {""};
    ($i:literal) => {concat!($i)};
    ($i:literal,) => {concat!($i)};
    ($f:literal,$($i:literal),+) => {concat!($f, $(",", $i),+)};
}

pub static ALL_FIELDS: &str = join_concat!(
    "AirTime",
    "Chapters",
    "ChildCount",
    "CumulativeRunTimeTicks",
    "DateCreated",
    "DateLastMediaAdded",
    "Etag",
    "ExternalUrls",
    "Genres",
    "ItemCounts",
    "MediaSources",
    "OriginalTitle",
    "Overview",
    "ParentId",
    "Path",
    "People",
    "ProductionLocations",
    "ProviderIds",
    "PrimaryImageAspectRatio",
    "Settings",
    "SeriesStudio",
    "SortName",
    "SpecialEpisodeNumbers",
    "Studios",
    "Tags",
    "MediaStreams",
    "SeasonUserData",
    "DateLastRefreshed",
    "DateLastSaved",
    "SpecialFeatureCount"
);

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetItemsQuery<'a> {
    pub user_id: Option<&'a str>,
    pub start_index: Option<u32>,
    pub limit: Option<u32>,
    pub parent_id: Option<&'a str>,
    pub adjacent_to: Option<&'a str>,
    pub exclude_item_types: Option<&'a str>,
    pub include_item_types: Option<&'a str>,
    pub enable_images: Option<bool>,
    pub enable_image_types: Option<&'a str>,
    pub image_type_limit: Option<u32>,
    pub enable_user_data: Option<bool>,
    pub fields: Option<&'a str>,
    pub sort_by: Option<&'a str>,
    pub recursive: Option<bool>,
    pub sort_order: Option<&'a str>,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetGenreQuery<'a> {
    pub start_index: Option<u32>,
    pub limit: Option<u32>,
    pub enable_images: Option<bool>,
    pub enable_image_types: Option<&'a str>,
    pub image_type_limit: Option<u32>,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetResumeQuery<'a> {
    pub user_id: Option<&'a str>,
    pub start_index: Option<u32>,
    pub limit: Option<u32>,
    pub search_term: Option<&'a str>,
    pub parent_id: Option<&'a str>,
    pub fields: Option<&'a str>,
    pub media_types: Option<&'a str>,
    pub enable_user_data: Option<bool>,
    pub image_type_limit: Option<u32>,
    pub enable_image_types: Option<&'a str>,
    pub exclude_item_types: Option<&'a str>,
    pub include_item_types: Option<&'a str>,
    pub enable_total_record_count: Option<bool>,
    pub enable_images: Option<bool>,
    pub exclude_active_sessions: Option<bool>,
}

#[derive(Debug, Default, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GetNextUpQuery<'a> {
    pub user_id: Option<&'a str>,
    pub start_index: Option<u32>,
    pub limit: Option<u32>,
    pub parent_id: Option<&'a str>,
    pub series_id: Option<&'a str>,
    pub fields: Option<&'a str>,
    pub enable_user_data: Option<bool>,
    pub image_type_limit: Option<u32>,
    pub enable_image_types: Option<&'a str>,
    pub next_up_date_cutoff: Option<&'a str>,
    pub enable_total_record_count: Option<bool>,
    pub enable_images: Option<bool>,
    pub disable_first_episode: Option<bool>,
    pub enable_resumable: Option<bool>,
    pub enable_rewatching: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub enum ImageType {
    Primary,
    Art,
    Backdrop,
    Banner,
    Logo,
    Thumb,
    Disc,
    Box,
    Screenshot,
    Menu,
    Chapter,
    BoxRear,
    Profile,
}
impl ImageType {
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Art => "Art",
            Self::Backdrop => "Backdrop",
            Self::Banner => "Banner",
            Self::Logo => "Logo",
            Self::Thumb => "Thumb",
            Self::Disc => "Disc",
            Self::Box => "Box",
            Self::Screenshot => "Screenshot",
            Self::Menu => "Menu",
            Self::Chapter => "Chapter",
            Self::BoxRear => "BoxRear",
            Self::Profile => "Profile",
        }
    }
}

impl Display for ImageType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Hash, Eq)]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub enum MediaType {
    Unknown,
    Video,
    Audio,
    Photo,
    Book,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, IntoStaticStr)]
#[serde(tag = "Type")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub enum ItemType {
    #[serde(rename_all = "PascalCase")]
    Movie,
    #[serde(rename_all = "PascalCase")]
    Episode {
        season_id: Option<String>,
        season_name: Option<String>,
        series_id: String,
        series_name: String,
    },
    #[serde(rename_all = "PascalCase")]
    Season {
        series_id: String,
        series_name: String,
    },
    MusicAlbum,
    Series,
    Playlist,
    Folder,
    CollectionFolder,
    Music {
        album_id: String,
        album: String,
    },
    Audio,
    #[serde(untagged)]
    Unknown {
        #[serde(rename = "Type")]
        item_type: String,
    },
}

impl ItemType {
    #[must_use]
    pub const fn is_single_media_item(&self) -> bool {
        match self {
            Self::Audio | Self::Movie | Self::Episode { .. } | Self::Music { .. } => true,
            Self::Season { .. }
            | Self::MusicAlbum
            | Self::Series
            | Self::Playlist
            | Self::Folder
            | Self::CollectionFolder
            | Self::Unknown { item_type: _ } => false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct UserData {
    pub playback_position_ticks: u64,
    pub unplayed_item_count: Option<u64>,
    pub is_favorite: bool,
    pub played: bool,
}

#[derive(Debug, Default, Clone, Copy, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SetUserData {
    pub playback_position_ticks: Option<u64>,
    pub unplayed_item_count: Option<u64>,
    pub is_favorite: Option<bool>,
    pub played: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct MediaItem {
    pub id: String,
    #[serde(default)]
    pub image_tags: HashMap<ImageType, String>,
    pub media_type: MediaType,
    pub name: String,
    pub sort_name: Option<String>,
    pub overview: Option<String>,
    pub user_data: Option<UserData>,
    #[serde(rename = "IndexNumber")]
    pub episode_index: Option<u64>,
    #[serde(rename = "ParentIndexNumber")]
    pub season_index: Option<u64>,
    pub run_time_ticks: Option<u64>,
    #[serde(default)]
    pub air_days: Vec<String>,
    #[serde(default)]
    pub chapters: Vec<Chapter>,
    #[serde(default)]
    pub child_count: u32,
    pub cumulative_runtime_ticks: Option<u64>,
    pub date_created: Option<String>,
    pub date_last_media_added: Option<String>,
    pub etag: Option<String>,
    #[serde(default)]
    pub external_urls: Vec<ExternalUrl>,
    #[serde(default)]
    pub genre_items: Vec<GenreItem>,
    pub original_language: Option<String>,
    pub original_title: Option<String>,
    pub parent_id: Option<String>,
    pub path: Option<String>,
    #[serde(default)]
    pub people: Vec<People>,
    pub premiere_date: Option<String>,
    pub primary_image_aspect_ratio: Option<f32>,
    pub production_year: Option<u16>,
    #[serde(default)]
    pub provider_ids: HashMap<String, String>,
    pub status: Option<String>,
    #[serde(default)]
    pub studios: Vec<Studio>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub item_type: ItemType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct Studio {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct People {
    pub id: String,
    pub name: String,
    pub role: Option<String>,
    #[serde(default)]
    pub image_tags: HashMap<ImageType, String>,
    #[serde(rename = "Type")]
    pub p_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct GenreItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub image_tags: HashMap<ImageType, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct ExternalUrl {
    pub name: String,
    pub url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct Chapter {
    pub start_position_ticks: Option<u64>,
    name: String,
    image_path: Option<String>,
    image_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub struct PlaybackInfo {
    pub media_sources: Vec<MediaSource>,
    pub play_session_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct RatingScore {
    pub score: u16,
    pub sub_score: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct ParentalRatingOption {
    pub name: String,
    pub value: Option<u16>,
    pub rating_score: Option<RatingScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct Country {
    pub name: String,
    pub display_name: String,
    pub two_letter_isoregion_name: Option<String>,
    #[serde(rename = "ThreeLetterISORegionName")]
    pub three_letter_iso_region_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct Culture {
    pub name: String,
    pub display_name: String,
    #[serde(rename = "TwoLetterISOLanguageName")]
    pub two_letter_iso_language_name: String,
    #[serde(rename = "ThreeLetterISOLanguageName")]
    pub three_letter_iso_language_name: String,
    #[serde(rename = "ThreeLetterISOLanguageNames")]
    pub three_letter_iso_language_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct MetadataEditor {
    #[serde(default)]
    pub parental_rating_options: Vec<ParentalRatingOption>,
    #[serde(default)]
    pub countries: Vec<Country>,
    #[serde(default)]
    pub cultures: Vec<Culture>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
#[serde(rename_all = "PascalCase")]
pub struct MetadataUpdate {
    pub name: String,
    pub original_title: String,
    pub sort_name: String,
}

impl JellyfinClient {
    #[instrument(skip(self))]
    pub async fn get_user_items_resume(
        &self,
        query: &GetResumeQuery<'_>,
    ) -> Result<JsonResponse<JellyfinVec<MediaItem>>> {
        self.send_request_json(self.get("/UserItems/Resume", query)?.empty_body()?)
            .await
    }

    #[instrument(skip(self))]
    pub async fn get_shows_next_up(
        &self,
        query: &GetNextUpQuery<'_>,
    ) -> Result<JsonResponse<JellyfinVec<MediaItem>>> {
        self.send_request_json(self.get("/Shows/NextUp", query)?.empty_body()?)
            .await
    }

    pub async fn get_items(
        &self,
        query: &GetItemsQuery<'_>,
    ) -> Result<JsonResponse<JellyfinVec<MediaItem>>> {
        self.send_request_json(self.get("/Items", query)?.empty_body()?)
            .await
    }

    pub async fn get_genre_items(
        &self,
        query: &GetGenreQuery<'_>,
    ) -> Result<JsonResponse<JellyfinVec<GenreItem>>> {
        self.send_request_json(self.get("/Genres", query)?.empty_body()?)
            .await
    }

    pub async fn refresh_item(&self, item: &str, query: &RefreshItemQuery) -> Result<()> {
        self.send_request(
            self.post(
                |base: &mut String| {
                    base.push_str("/Items/");
                    base.push_str(item);
                    base.push_str("/Refresh");
                },
                query,
            )?
            .empty_body()?,
        )
        .await?;
        Ok(())
    }

    pub async fn get_item(
        &self,
        id: &str,
        user_id: Option<&str>,
    ) -> Result<JsonResponse<MediaItem>> {
        self.send_request_json(
            self.get(
                |prefix: &mut String| {
                    prefix.push_str("/Items/");
                    prefix.push_str(id);
                },
                &UserIdQuery { user_id },
            )?
            .empty_body()?,
        )
        .await
    }

    pub async fn set_user_data(&self, item: &str, data: &SetUserData) -> Result<()> {
        self.send_request(
            self.post(
                |prefix: &mut String| {
                    prefix.push_str("/Items/");
                    prefix.push_str(item);
                    prefix.push_str("/UserData");
                },
                NoQuery,
            )?
            .json_body(data)?,
        )
        .await?;
        Ok(())
    }

    pub fn get_download_uri(&self, item_id: &str) -> Result<Uri> {
        Uri::builder()
            .scheme(if self.config.tls { "https" } else { "http" })
            .authority(self.config.authority.clone())
            .path_and_query(self.build_uri(
                |prefix: &mut String| {
                    prefix.push_str("/Items/");
                    prefix.push_str(item_id);
                    prefix.push_str("/Download");
                },
                DownloadQuery {
                    api_key: self.get_auth().token(),
                },
            )?)
            .build()
            .context("assembling video uri")
    }

    pub fn get_playback_uri(&self, item: &MediaItem) -> Result<Uri> {
        let uri = match &item.item_type {
            ItemType::Movie | ItemType::Episode { .. } => self.get_video_uri(&item.id),
            ItemType::Music { .. } | ItemType::Audio => self.get_audio_uri(&item.id),
            ItemType::Season { .. }
            | ItemType::MusicAlbum
            | ItemType::Series
            | ItemType::Playlist
            | ItemType::Folder
            | ItemType::CollectionFolder => Err(eyre!(
                "item type {} is not itself playable",
                <&str>::from(&item.item_type)
            )),
            ItemType::Unknown { item_type } => Err(eyre!("unsupported item type {item_type}")),
        }?;
        debug!(%uri,"constructed playback uri");
        Ok(uri)
    }

    pub fn get_audio_uri(&self, item_id: &str) -> Result<Uri> {
        Uri::builder()
            .scheme(if self.config.tls { "https" } else { "http" })
            .authority(self.config.authority.clone())
            .path_and_query(self.build_uri(
                |prefix: &mut String| {
                    prefix.push_str("/Audio/");
                    prefix.push_str(item_id);
                    prefix.push_str("/stream");
                },
                GetAudioQuery {
                    use_original: "true",
                    play_session_id: &self.get_auth().session_id,
                    api_key: self.get_auth().token(),
                    device_id: self.get_auth().device_id(),
                },
            )?)
            .build()
            .context("assembling audio uri")
    }

    pub fn get_video_uri(&self, item_id: &str) -> Result<Uri> {
        Uri::builder()
            .scheme(if self.config.tls { "https" } else { "http" })
            .authority(self.config.authority.clone())
            .path_and_query(self.build_uri(
                |prefix: &mut String| {
                    prefix.push_str("/videos/");
                    prefix.push_str(item_id);
                    prefix.push_str("/stream");
                },
                GetVideoQuery {
                    use_original: "true",
                    media_source_id: item_id,
                    play_session_id: &self.get_auth().session_id,
                    api_key: self.get_auth().token(),
                    device_id: self.get_auth().device_id(),
                },
            )?)
            .build()
            .context("assembling video uri")
    }

    pub fn get_subtitle_uri(
        &self,
        item_id: &str,
        media_source_id: &str,
        index: i32,
        format: &str,
    ) -> Result<Uri> {
        Uri::builder()
            .scheme(if self.config.tls { "https" } else { "http" })
            .authority(self.config.authority.clone())
            .path_and_query(self.build_uri(
                |prefix: &mut String| {
                    prefix.push_str("/Videos/");
                    prefix.push_str(item_id);
                    prefix.push('/');
                    prefix.push_str(media_source_id);
                    prefix.push_str("/Subtitles/");
                    prefix.push_str(&index.to_string());
                    prefix.push_str("/0/Stream.");
                    prefix.push_str(format);
                },
                DownloadQuery {
                    api_key: self.get_auth().token(),
                },
            )?)
            .build()
            .context("assembling subtitle uri")
    }

    pub async fn get_playback_info(&self, item_id: &str) -> Result<JsonResponse<PlaybackInfo>> {
        self.send_request_json(
            self.get(
                |prefix: &mut String| {
                    prefix.push_str("/Items/");
                    prefix.push_str(item_id);
                    prefix.push_str("/PlaybackInfo");
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await
    }
    pub async fn metadata_editor(&self, item_id: &str) -> Result<JsonResponse<MetadataEditor>> {
        self.send_request_json(
            self.get(
                |prefix: &mut String| {
                    prefix.push_str("/Items/");
                    prefix.push_str(item_id);
                    prefix.push_str("/MetadataEditor");
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await
    }
    pub async fn update_item(&self, item_id: &str, new_metadata: &MetadataUpdate) -> Result<()> {
        self.send_request(
            self.post(
                |prefix: &mut String| {
                    prefix.push_str("/Items/");
                    prefix.push_str(item_id);
                },
                NoQuery,
            )?
            .json_body(new_metadata)?,
        )
        .await?;
        Ok(())
    }
}

impl JellyfinClient {
    pub async fn set_unplayed(&self, item: &str) -> Result<()> {
        self.send_request(
            self.delete(
                |prefix: &mut String| {
                    prefix.push_str("/Users/");
                    prefix.push_str(&self.get_auth().user.id);
                    prefix.push_str("/PlayedItems/");
                    prefix.push_str(item);
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await?;
        Ok(())
    }
    pub async fn set_played(&self, item: &str) -> Result<()> {
        self.send_request(
            self.post(
                |prefix: &mut String| {
                    prefix.push_str("/Users/");
                    prefix.push_str(&self.get_auth().user.id);
                    prefix.push_str("/PlayedItems/");
                    prefix.push_str(item);
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await?;
        Ok(())
    }
}
