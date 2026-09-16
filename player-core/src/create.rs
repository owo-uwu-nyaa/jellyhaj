use std::{
    ffi::CString,
    path::Path,
    sync::{Arc, atomic::AtomicBool},
    time::Duration,
};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt, eyre},
};
use jellyfin::{
    JellyfinClient,
    items::{ItemType, MediaItem},
};
use jellyhaj_core::state::NextScreen;
use mpv_async::{Mpv, mpv_node_list, mpv_node_map};
use spawn::Spawner;
use tokio::{
    sync::{
        broadcast,
        mpsc::{self, UnboundedSender},
    },
    time::{MissedTickBehavior, interval},
};
use tokio_util::sync::CancellationToken;
use tracing::{debug, error_span, instrument};

use crate::{
    OwnedPlayerHandle, PlayerHandle, PlaylistItem, PlaylistItemIdGen, mpv_stream::MpvStream,
    poll::PollState,
};

impl OwnedPlayerHandle {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        jellyfin: JellyfinClient,
        hwdec: &str,
        profiles: &[String],
        log_level: &str,
        mpv_config_file: Option<&Path>,
        minimized: bool,
        spawn: &Spawner,
        widget_sender: UnboundedSender<NextScreen>,
    ) -> Result<Self> {
        let mpv = MpvStream::new(
            &jellyfin,
            hwdec,
            mpv_config_file
                .map(|c| CString::new(c.as_os_str().as_encoded_bytes()))
                .transpose()?
                .as_deref(),
            profiles,
            log_level,
            minimized,
        )?;
        let mut position_send_timer = interval(Duration::from_secs(1));
        position_send_timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
        let playlist = Arc::new(Vec::new());
        let (c_send, c_recv) = mpsc::unbounded_channel();
        let (send_events, _) = broadcast::channel(32);
        let stop = CancellationToken::new();

        spawn.spawn(
            PollState {
                idle: true,
                mpv,
                commands: c_recv,
                position_send_timer,
                paused: false,
                position: 0.0,
                duration: 0.0,
                speed: 1.0,
                volume: 100,
                index: None,
                fullscreen: true,
                stop: stop.clone().cancelled_owned(),
                jellyfin,
                playlist,
                playlist_id_gen: PlaylistItemIdGen::default(),
                minimized,
                seeked: false,
                send_events,
                widget_sender,
            },
            error_span!("mpv-player"),
            "mpv-player",
        );
        Ok(Self {
            inner: PlayerHandle {
                closed: Arc::new(AtomicBool::new(false)),
                send: c_send,
            },
            _stop: stop.drop_guard(),
        })
    }
}

#[instrument(skip_all)]
pub fn set_playlist(
    mpv: &Mpv,
    jellyfin: &JellyfinClient,
    id_gen: &mut PlaylistItemIdGen,
    items: Vec<MediaItem>,
    index: usize,
) -> Result<Vec<Arc<PlaylistItem>>> {
    let position = items[index]
        .user_data
        .as_ref()
        .ok_or_eyre("user data missing")?
        .playback_position_ticks
        / 10_000_000;

    for item in &items[0..index] {
        append(mpv, jellyfin, item)?;
    }
    debug!("previous files added");
    let item = &items[index];
    let uri = jellyfin.get_playback_uri(item)?.to_string();
    debug!("adding {uri} to queue and play it");
    mpv_node_map!(opt;{
        c"start": &CString::new(position.to_string()).context("converting start to cstr")?,
        c"force-media-title": &name(item)?
    });
    mpv_node_list!(cmd;[
        c"loadfile",
        &CString::new(uri).context("converting video url to cstr")?,
        c"append-play",
        0i64,
        &opt
    ]);
    mpv.command(&cmd).context("added main item")?;
    debug!("main file added to playlist at index {index}");
    for item in &items[index + 1..] {
        append(mpv, jellyfin, item)?;
    }
    debug!("later files added");
    Ok(items
        .into_iter()
        .map(|item| {
            Arc::new(PlaylistItem {
                item,
                id: id_gen.next(),
            })
        })
        .collect())
}

#[instrument(skip_all)]
fn append(mpv: &Mpv, jellyfin: &JellyfinClient, item: &MediaItem) -> Result<()> {
    let uri = jellyfin.get_playback_uri(item)?.to_string();
    debug!("adding {uri} to queue");
    mpv_node_map!(opt; {c"force-media-title": &name(item)?});
    mpv_node_list!(cmd; [
        c"loadfile",
        &CString::new(uri).context("converting video url to cstr")?,
        c"append",
        0i64,
        &opt
    ]);
    mpv.command(&cmd)?;
    Ok(())
}

#[instrument(skip_all)]
fn name(item: &MediaItem) -> Result<CString> {
    let name = match &item.item_type {
        ItemType::Music {
            album_id: _,
            album: _,
        }
        | ItemType::Movie
        | ItemType::Audio => item.name.clone(),
        ItemType::Episode {
            season_id: _,
            season_name: _,
            series_id: _,
            series_name,
        } => {
            if let Some(i) = item.episode_index {
                let index = i.to_string();
                //dumb check if name is usefull
                let (mut string, episode) = if item.name.contains(&index) {
                    (series_name.clone(), false)
                } else {
                    (item.name.clone(), true)
                };
                string.push(' ');
                if episode {
                    string.push('(');
                }
                if let Some(i) = item.season_index {
                    string.push('S');
                    string += &i.to_string();
                }
                string.push('E');
                string += &index;
                if episode {
                    string.push(')');
                }
                string
            } else {
                item.name.clone()
            }
        }
        t => return Err(eyre!("unsupported item type: {t:?}")),
    };
    Ok(CString::new(name)?)
}
