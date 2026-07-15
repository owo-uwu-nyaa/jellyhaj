use std::collections::HashMap;

use jellyfin::{JellyfinClient, connect::JsonResponseHelper};
use player_core::{Command, PlayItem, PlayerHandle, PlaylistItem, state::SharedPlayerState};
use tokio::try_join;
use zbus::{
    fdo::{Error, Result},
    interface,
    object_server::SignalEmitter,
    zvariant::{ObjectPath, OwnedObjectPath},
};

use crate::types::{Metadata, parse_track_id, track_id_as_object};

pub struct TrackList {
    player: PlayerHandle,
    jellyfin: JellyfinClient,
    state: SharedPlayerState,
}

impl TrackList {
    pub const fn new(
        player: PlayerHandle,
        jellyfin: JellyfinClient,
        state: SharedPlayerState,
    ) -> Self {
        Self {
            player,
            jellyfin,
            state,
        }
    }
}
#[allow(clippy::needless_pass_by_value, clippy::unused_self)]
#[interface(name = "org.mpris.MediaPlayer2.TrackList", spawn = true)]
impl TrackList {
    fn get_track_metadata(&self, ids: Vec<ObjectPath<'_>>) -> Result<Vec<Metadata>> {
        let state = self.state.lock();
        let indexes: HashMap<_, _> = state
            .playlist
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id, index))
            .collect();
        let mut res = Vec::with_capacity(ids.len());
        for id in ids {
            let id = parse_track_id(&id)?
                .ok_or_else(|| Error::InvalidArgs("NoTrack can have no Metadata".to_string()))?;
            let index = *indexes.get(&id).ok_or_else(|| {
                Error::InvalidArgs(format!("{} is currently not in the track list", id.id))
            })?;
            let item: &PlaylistItem = &state.playlist[index];
            res.push(Metadata::new(item, &self.jellyfin));
        }
        Ok(res)
    }
    async fn add_track(
        &self,
        uri: &str,
        after: ObjectPath<'_>,
        set_as_current: bool,
    ) -> Result<()> {
        if URI_PREFIX != &uri[..URI_PREFIX.len()] {
            return Err(Error::InvalidArgs(format!("unsupported uri scheme: {uri}")));
        }
        let after = parse_track_id(&after)?;
        let item = &uri[URI_PREFIX.len()..];
        let (item, playback_info) = try_join!(
            self.jellyfin.get_item(item, None).deserialize(),
            self.jellyfin.get_playback_info(item).deserialize()
        )
        .map_err(|r| Error::Failed(r.to_string()))?;
        if !item.item_type.is_single_media_item() {
            return Err(Error::InvalidArgs(format!(
                "Item {item:?} is not a single playable piece of media but some collection"
            )));
        }
        self.player.send(Command::AddTrack {
            item: Box::new(PlayItem {
                item,
                playback_session_id: playback_info.play_session_id,
            }),
            after,
            play: set_as_current,
        });

        Ok(())
    }
    fn remove_track(&self, track: ObjectPath<'_>) -> Result<()> {
        let id = parse_track_id(&track)?
            .ok_or_else(|| Error::InvalidArgs("NoTrack can not be removed".to_string()))?;
        self.player.send(Command::Remove(id));
        Ok(())
    }

    fn go_to(&self, track: ObjectPath<'_>) -> Result<()> {
        let id = parse_track_id(&track)?
            .ok_or_else(|| Error::InvalidArgs("NoTrack can not be played".to_string()))?;
        self.player.send(Command::Play(id));
        Ok(())
    }

    #[zbus(signal)]
    pub async fn track_list_replaced(
        emitter: &SignalEmitter<'_>,
        tracks: Vec<OwnedObjectPath>,
        current_track: ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn track_added(
        emitter: &SignalEmitter<'_>,
        metadata: Metadata,
        after_track: ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    pub async fn track_removed(
        emitter: &SignalEmitter<'_>,
        track_id: ObjectPath<'_>,
    ) -> zbus::Result<()>;

    #[zbus(property(emits_changed_signal = "invalidates"))]
    fn tracks(&self) -> Vec<OwnedObjectPath> {
        self.state
            .lock()
            .playlist
            .iter()
            .map(|i| track_id_as_object(Some(i.id)))
            .collect()
    }
    #[zbus(property(emits_changed_signal = "const"))]
    const fn can_edit_tracks(&self) -> bool {
        true
    }
}

const URI_PREFIX: &str = "jellyfin-item:";
