use std::{
    ffi::{CStr, CString},
    ops::Deref,
    task::{Poll, ready},
};

use color_eyre::eyre::{Context, Result};
use futures_util::Stream;
use jellyfin::JellyfinClient;
use mpv_async::{
    Mpv,
    events::MpvEvent,
    nodes::{MpvFormat, ToMpvNode, mpv_node_list},
};
use pin_project_lite::pin_project;
use tracing::{info, instrument, trace, warn};

use super::log::log_message;

#[derive(Debug)]
pub enum ObservedProperty {
    Position(f64),
    Duration(f64),
    Idle(bool),
    Pause(bool),
    Fullscreen(bool),
    Minimized(bool),
    PlaylistPos(i64),
    Volume(i64),
    Speed(f64),
}

#[derive(Debug)]
pub enum ClientCommand {
    Stop,
    Unpause,
}

#[derive(Debug)]
pub enum Event {
    PropertyChanged(ObservedProperty),
    Command(ClientCommand),
    Seek,
}

pin_project! {
    pub struct MpvStream {
        #[pin]
        mpv: Mpv,
    }
}

impl Deref for MpvStream {
    type Target = Mpv;
    fn deref(&self) -> &Self::Target {
        &self.mpv
    }
}

impl Stream for MpvStream {
    type Item = Result<Event>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        let mut this = self.project();
        Poll::Ready(loop {
            let event = match ready!(this.mpv.poll_wait_event(cx)).context("waiting for mpv events")
            {
                Err(e) => break Some(Err(e)),
                Ok(v) => v,
            };
            trace!(?event);
            match event {
                MpvEvent::Shutdown => {
                    info!("shutdown request received");
                    break None;
                }
                MpvEvent::LogMessage {
                    prefix,
                    level,
                    level_str: _,
                    text,
                } => log_message(prefix, level, text),
                MpvEvent::ClientMessage(client_message) => {
                    if client_message.len() == 1 && client_message[0] == c"stop-player" {
                        break Some(Ok(Event::Command(ClientCommand::Stop)));
                    }
                    warn!(?client_message, "received unknown client message");
                }
                MpvEvent::Seek => break Some(Ok(Event::Seek)),
                MpvEvent::PropertyChange { userdata, data } => {
                    trace!(?data, "received property");
                    match userdata {
                        1 => {
                            assert_eq!(data.name, c"time-pos");
                            if let Some(data) = data.differentiate().float() {
                                break Some(Ok(Event::PropertyChanged(
                                    ObservedProperty::Position(data),
                                )));
                            }
                        }
                        2 => {
                            assert_eq!(data.name, c"idle-active");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::Idle(
                                data.differentiate().bool().expect("wrong type"),
                            ))));
                        }
                        3 => {
                            assert_eq!(data.name, c"pause");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::Pause(
                                data.differentiate().bool().expect("wrong type"),
                            ))));
                        }
                        4 => {
                            assert_eq!(data.name, c"fullscreen");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::Fullscreen(
                                data.differentiate().bool().expect("wrong type"),
                            ))));
                        }
                        5 => {
                            assert_eq!(data.name, c"window-minimized");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::Minimized(
                                data.differentiate().bool().expect("wrong type"),
                            ))));
                        }
                        6 => {
                            assert_eq!(data.name, c"playlist-pos");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::PlaylistPos(
                                data.differentiate().int().expect("wrong type"),
                            ))));
                        }
                        7 => {
                            assert_eq!(data.name, c"speed");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::Speed(
                                data.differentiate().float().expect("wrong type"),
                            ))));
                        }
                        8 => {
                            assert_eq!(data.name, c"volume");
                            break Some(Ok(Event::PropertyChanged(ObservedProperty::Volume(
                                data.differentiate().int().expect("wrong type"),
                            ))));
                        }
                        9 => {
                            assert_eq!(data.name, c"duration");
                            if let Some(data) = data.differentiate().float() {
                                break Some(Ok(Event::PropertyChanged(
                                    ObservedProperty::Duration(data),
                                )));
                            }
                        }
                        _ => {
                            warn!("unknown property observation: {:?}", data.name);
                        }
                    }
                }
                MpvEvent::QueueOverflow => {
                    warn!("queue overflow");
                }
                MpvEvent::CommandReply { userdata, data: _ } => match userdata {
                    0 => {}
                    1 => break Some(Ok(Event::Command(ClientCommand::Unpause))),
                    v => warn!("inknown command reply {v}"),
                },
                _ => {}
            }
        })
    }
}

impl MpvStream {
    #[instrument(skip_all, name = "new_mpv_stream")]
    pub fn new(
        jellyfin: &JellyfinClient,
        hwdec: &str,
        mpv_config_file: Option<&CStr>,
        profiles: &[String],
        log_level: &str,
        minimized: bool,
    ) -> Result<Self> {
        let mpv = Mpv::new()?;
        mpv.set_property(c"title", c"jellyhaj-player")?;
        mpv.set_property(c"fullscreen", true)?;
        mpv.set_property(c"window-minimized", minimized)?;
        mpv.set_property(c"drag-and-drop", false)?;
        mpv.set_property(c"osc", true)?;
        mpv.set_property(c"vo", c"gpu-next")?;
        mpv.set_property(c"terminal", false)?;
        let mut header = b"authorization: ".to_vec();
        header.extend_from_slice(jellyfin.get_auth().header.as_bytes());
        mpv_node_list!(header;[&CString::new(header)
                .context("converting auth header to cstr")?]);
        mpv.set_property(c"http-header-fields", &header.node())?;
        mpv.set_property(c"input-default-bindings", true)?;
        mpv.set_property(c"input-vo-keyboard", true)?;
        mpv.set_property(
            c"hwdec",
            CString::new(hwdec)
                .context("converting hwdec to cstr")?
                .as_c_str(),
        )?;
        mpv.set_property(c"idle", c"yes")?;
        if let Some(config) = mpv_config_file {
            mpv_node_list!(cmd;[c"load-config-file", config]);
            mpv.command(&cmd)?;
        }
        let profiles: Vec<CString> = profiles
            .iter()
            .map(|s| CString::new(s.as_str()))
            .collect::<std::result::Result<_, _>>()?;
        let profiles: Vec<_> = profiles
            .iter()
            .map(mpv_async::nodes::ToMpvNode::node)
            .collect();

        mpv.set_property(
            c"profile",
            &mpv_async::nodes::MpvNodeList::new(&profiles).node(),
        )?;
        let mpv = mpv.initialize()?;
        mpv.request_log_messages(
            &CString::new(log_level).context("converting log level to cstr")?,
        )?;
        //mpv.enable_event(mpv_event_id::PropertyChange)?;
        //mpv.enable_event(mpv_event_id::LogMessage)?;
        //mpv.enable_event(mpv_event_id::QueueOverflow)?;
        //mpv.enable_event(mpv_event_id::ClientMessage)?;
        //mpv.enable_event(mpv_event_id::Seek)?;
        mpv.observe_property(1, c"time-pos", MpvFormat::Double)?;
        mpv.observe_property(2, c"idle-active", MpvFormat::Flag)?;
        mpv.observe_property(3, c"pause", MpvFormat::Flag)?;
        mpv.observe_property(4, c"fullscreen", MpvFormat::Flag)?;
        mpv.observe_property(5, c"window-minimized", MpvFormat::Flag)?;
        mpv.observe_property(6, c"playlist-pos", MpvFormat::Int64)?;
        mpv.observe_property(7, c"speed", MpvFormat::Double)?;
        mpv.observe_property(8, c"volume", MpvFormat::Int64)?;
        mpv.observe_property(9, c"duration", MpvFormat::Double)?;
        mpv.command(&[
            c"keybind".node(),
            c"q".node(),
            stop_cmd(mpv.client_name()).node(),
            c"on quit stop the player instead".node(),
        ])?;
        info!("mpv initialized");
        Ok(Self { mpv })
    }
}

fn stop_cmd(name: &CStr) -> CString {
    let name = name.to_bytes();
    let first = b"script-message-to ";
    let end = c" stop-player".to_bytes_with_nul();
    let mut vec = Vec::with_capacity(first.len() + name.len() + end.len());
    vec.extend_from_slice(first);
    vec.extend_from_slice(name);
    vec.extend_from_slice(end);
    CString::from_vec_with_nul(vec).expect("constructed with null byte")
}
