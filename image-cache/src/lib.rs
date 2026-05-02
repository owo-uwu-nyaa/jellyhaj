use std::{borrow::Borrow, collections::HashMap, fmt::Debug, hash::Hash, sync::Arc};

use jellyfin::items::ImageType;
use parking_lot::Mutex;
use ratatui_image::protocol::Protocol;
use tracing::{instrument, trace};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ImageSize {
    pub p_width: u32,
    pub p_height: u32,
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub struct ImageProtocolKey {
    pub image_type: ImageType,
    pub item_id: String,
    pub tag: String,
    pub size: ImageSize,
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub struct ImageProtocolKeyRef<'s> {
    pub image_type: ImageType,
    pub item_id: &'s str,
    pub tag: &'s str,
    pub size: ImageSize,
}

impl<'s> ImageProtocolKeyRef<'s> {
    #[must_use]
    pub const fn new(
        image_type: ImageType,
        item_id: &'s str,
        tag: &'s str,
        size: ImageSize,
    ) -> Self {
        Self {
            image_type,
            item_id,
            tag,
            size,
        }
    }
}

pub trait AsKeyRef {
    fn as_key_ref(&self) -> ImageProtocolKeyRef<'_>;
}
impl AsKeyRef for ImageProtocolKey {
    fn as_key_ref(&self) -> ImageProtocolKeyRef<'_> {
        ImageProtocolKeyRef {
            image_type: self.image_type,
            item_id: &self.item_id,
            tag: &self.tag,
            size: self.size,
        }
    }
}
impl AsKeyRef for ImageProtocolKeyRef<'_> {
    fn as_key_ref(&self) -> ImageProtocolKeyRef<'_> {
        *self
    }
}

impl PartialEq for dyn AsKeyRef + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.as_key_ref() == other.as_key_ref()
    }
}
impl Eq for dyn AsKeyRef + '_ {}
impl Hash for dyn AsKeyRef + '_ {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.as_key_ref().hash(state);
    }
}
impl Debug for dyn AsKeyRef + '_ {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&self.as_key_ref(), f)
    }
}
impl<'s> Borrow<dyn AsKeyRef + 's> for ImageProtocolKey {
    fn borrow(&self) -> &(dyn AsKeyRef + 's) {
        self
    }
}

#[derive(Clone)]
pub struct ImageProtocolCache {
    protocols: Arc<Mutex<HashMap<ImageProtocolKey, Protocol>>>,
}

impl ImageProtocolCache {
    #[instrument(level = "trace", skip(self))]
    pub fn remove(&self, key: &dyn AsKeyRef) -> Option<Protocol> {
        trace!("storing image protocol in cache");
        self.protocols.lock().remove(key)
    }
    #[instrument(level = "trace", skip(self, protocol))]
    pub fn store(&self, protocol: Protocol, key: ImageProtocolKey) {
        trace!("storing image protocol in cache");
        self.protocols.lock().insert(key, protocol);
    }
    #[must_use]
    pub fn new() -> Self {
        Self {
            protocols: Arc::new(Mutex::new(HashMap::new())),
        }
    }
}

impl Default for ImageProtocolCache {
    fn default() -> Self {
        Self::new()
    }
}
