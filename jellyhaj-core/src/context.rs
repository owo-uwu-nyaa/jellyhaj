use std::sync::Arc;

pub use ::keybinds::KeybindEvents;
pub use config::Config;
pub use image_cache::ImageProtocolCache;
pub use jellyfin::{Auth, JellyfinClient};
pub use jellyhaj_event_listener::JellyfinEventInterests;
pub use jellyhaj_widgets_core::ContextRef;
pub use player_core::{OwnedPlayerHandle, PlayerHandle};
pub use ratatui::DefaultTerminal;
pub use ratatui_image::picker::Picker;
pub use spawn::Spawner;
use sqlx::SqliteConnection;
use stats_data::StatsData;
use tokio::sync::Mutex;

pub use stats_data::Stats;
pub type DBInner = Mutex<SqliteConnection>;
pub type DB = Arc<DBInner>;
pub type ImagePicker = Arc<Picker>;

#[derive(Clone)]
pub struct TuiContext {
    pub jellyfin: JellyfinClient,
    pub jellyfin_events: JellyfinEventInterests,
    pub config: Arc<Config>,
    pub image_picker: ImagePicker,
    pub cache: DB,
    pub image_cache: ImageProtocolCache,
    pub mpv_handle: PlayerHandle,
    pub stats: Stats,
    pub spawn: Spawner,
}

impl ContextRef<JellyfinClient> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &JellyfinClient {
        &self.jellyfin
    }
}
impl ContextRef<JellyfinEventInterests> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &JellyfinEventInterests {
        &self.jellyfin_events
    }
}
impl ContextRef<Arc<Config>> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &Arc<Config> {
        &self.config
    }
}
impl ContextRef<Config> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &Config {
        &self.config
    }
}
impl ContextRef<ImagePicker> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &ImagePicker {
        &self.image_picker
    }
}
impl ContextRef<Picker> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &Picker {
        &self.image_picker
    }
}
impl ContextRef<DB> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &DB {
        &self.cache
    }
}
impl ContextRef<DBInner> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &DBInner {
        &self.cache
    }
}
impl ContextRef<ImageProtocolCache> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &ImageProtocolCache {
        &self.image_cache
    }
}

impl ContextRef<PlayerHandle> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &PlayerHandle {
        &self.mpv_handle
    }
}

impl ContextRef<Stats> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &Stats {
        &self.stats
    }
}

impl ContextRef<StatsData> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &StatsData {
        &self.stats
    }
}

impl ContextRef<Spawner> for TuiContext {
    #[inline]
    fn as_ref(&self) -> &Spawner {
        &self.spawn
    }
}
