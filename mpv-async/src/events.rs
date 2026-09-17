use core::slice;
use std::{
    ffi::{CStr, c_char, c_int, c_void},
    fmt::Debug,
    marker::PhantomData,
    ops::Index,
    ptr::NonNull,
};

use crate::{Result, get_err_reply};
use mpv_sys::{
    mpv_end_file_reason, mpv_error, mpv_event, mpv_event_client_message, mpv_event_end_file,
    mpv_event_hook, mpv_event_id, mpv_event_log_message, mpv_event_property, mpv_event_start_file,
    mpv_format, mpv_handle, mpv_hook_continue, mpv_log_level,
};

#[cfg(feature = "macros")]
#[doc(hidden)]
pub use mpv_async_macros as macros;

use crate::nodes::{MpvNode, MpvNodeRef};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpvLogLevel {
    Fatal,
    Error,
    Warn,
    Info,
    Verbose,
    Debug,
    Trace,
    None,
    Unknown,
}

impl MpvLogLevel {
    #[must_use]
    pub const fn to_cstr(self) -> Option<&'static CStr> {
        Some(match self {
            Self::Fatal => c"fatal",
            Self::Error => c"error",
            Self::Warn => c"warn",
            Self::Info => c"info",
            Self::Verbose => c"v",
            Self::Debug => c"debug",
            Self::Trace => c"trace",
            Self::None => c"no",
            Self::Unknown => return None,
        })
    }
}

#[cfg(feature = "tracing")]
static SEEN_UNKNOWN_LOG_LEVEL: std::sync::RwLock<Vec<mpv_log_level>> =
    std::sync::RwLock::new(Vec::new());

impl From<mpv_log_level> for MpvLogLevel {
    fn from(value: mpv_log_level) -> Self {
        match value {
            mpv_log_level::MPV_LOG_LEVEL_FATAL => Self::Fatal,
            mpv_log_level::MPV_LOG_LEVEL_ERROR => Self::Error,
            mpv_log_level::MPV_LOG_LEVEL_WARN => Self::Warn,
            mpv_log_level::MPV_LOG_LEVEL_INFO => Self::Info,
            mpv_log_level::MPV_LOG_LEVEL_V => Self::Verbose,
            mpv_log_level::MPV_LOG_LEVEL_DEBUG => Self::Debug,
            mpv_log_level::MPV_LOG_LEVEL_TRACE => Self::Trace,
            mpv_log_level::MPV_LOG_LEVEL_NONE => Self::None,
            v => {
                #[cfg(feature = "tracing")]
                let seen = SEEN_UNKNOWN_LOG_LEVEL
                    .read()
                    .expect("should never panic")
                    .contains(&v);
                #[cfg(feature = "tracing")]
                if !seen {
                    SEEN_UNKNOWN_LOG_LEVEL
                        .write()
                        .expect("should never panic")
                        .push(v);
                    tracing::warn!("unknown log level: {}", v.0);
                }
                let _ = v;
                Self::Unknown
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MpvEventId {
    Shutdown,
    LogMessage,
    GetPropertyReply,
    SetPropertyReply,
    CommandReply,
    StartFile,
    EndFile,
    FileLoaded,
    ClientMessage,
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    PropertyChange,
    QueueOverflow,
    Hook,
    None,
}

impl MpvEventId {
    #[must_use]
    pub const fn to_raw(self) -> mpv_event_id {
        match self {
            Self::None => mpv_event_id::MPV_EVENT_NONE,
            Self::Shutdown => mpv_event_id::MPV_EVENT_SHUTDOWN,
            Self::LogMessage => mpv_event_id::MPV_EVENT_LOG_MESSAGE,
            Self::GetPropertyReply => mpv_event_id::MPV_EVENT_GET_PROPERTY_REPLY,
            Self::SetPropertyReply => mpv_event_id::MPV_EVENT_SET_PROPERTY_REPLY,
            Self::CommandReply => mpv_event_id::MPV_EVENT_COMMAND_REPLY,
            Self::StartFile => mpv_event_id::MPV_EVENT_START_FILE,
            Self::EndFile => mpv_event_id::MPV_EVENT_END_FILE,
            Self::FileLoaded => mpv_event_id::MPV_EVENT_FILE_LOADED,
            Self::ClientMessage => mpv_event_id::MPV_EVENT_CLIENT_MESSAGE,
            Self::VideoReconfig => mpv_event_id::MPV_EVENT_VIDEO_RECONFIG,
            Self::AudioReconfig => mpv_event_id::MPV_EVENT_AUDIO_RECONFIG,
            Self::Seek => mpv_event_id::MPV_EVENT_SEEK,
            Self::PlaybackRestart => mpv_event_id::MPV_EVENT_PLAYBACK_RESTART,
            Self::PropertyChange => mpv_event_id::MPV_EVENT_PROPERTY_CHANGE,
            Self::QueueOverflow => mpv_event_id::MPV_EVENT_QUEUE_OVERFLOW,
            Self::Hook => mpv_event_id::MPV_EVENT_HOOK,
        }
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub enum MpvEvent<'s> {
    Shutdown,
    LogMessage {
        prefix: &'s CStr,
        level: MpvLogLevel,
        level_str: &'s CStr,
        text: &'s CStr,
    },
    GetPropertyReply {
        userdata: u64,
        data: MpvProperty<'s>,
    },
    SetPropertyReply {
        userdata: u64,
    },
    CommandReply {
        userdata: u64,
        data: &'s MpvNode,
    },
    StartFile {
        playlist_entry_id: i64,
    },
    EndFile {
        entry_id: i64,
        reason: EndFileReason,
    },
    FileLoaded,
    ClientMessage(ClientMessage<'s>),
    VideoReconfig,
    AudioReconfig,
    Seek,
    PlaybackRestart,
    PropertyChange {
        userdata: u64,
        data: MpvProperty<'s>,
    },
    QueueOverflow,
    Hook {
        name: &'s CStr,
        handle: HookHandle<'s>,
        userdata: u64,
    },
    Unknown,
}

#[non_exhaustive]
#[derive(Debug)]
pub enum EndFileReason {
    Error(mpv_error),
    EOF,
    Stop,
    Quit,
    Redirect { insert_id: i64, num: c_int },
    Unknown,
}

pub struct MpvProperty<'s> {
    pub name: &'s CStr,
    pub value: *const c_void,
    pub format: mpv_format,
}

impl<'s> MpvProperty<'s> {
    #[must_use]
    const unsafe fn new(val: &mpv_event_property) -> Self {
        Self {
            name: unsafe { CStr::from_ptr(val.name) },
            value: val.data,
            format: val.format,
        }
    }
    pub fn differentiate(&self) -> MpvNodeRef<'s> {
        unsafe { MpvNodeRef::from_property_ptr(self.format, self.value) }
    }
}

impl Debug for MpvProperty<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MpvProperty")
            .field("name", &self.name)
            .field("value", &self.differentiate())
            .finish()
    }
}

#[derive(Clone, Copy)]
pub struct ClientMessage<'s> {
    args: &'s [*const c_char],
}

impl ClientMessage<'_> {
    #[must_use]
    pub const fn len(&self) -> usize {
        self.args.len()
    }
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.args.is_empty()
    }
}

impl Debug for ClientMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

impl<'s> IntoIterator for ClientMessage<'s> {
    type Item = &'s CStr;

    type IntoIter =
        std::iter::Map<std::slice::Iter<'s, *const c_char>, fn(&*const c_char) -> &'s CStr>;

    fn into_iter(self) -> Self::IntoIter {
        self.args.iter().map(|p| unsafe { CStr::from_ptr(*p) })
    }
}

impl Index<usize> for ClientMessage<'_> {
    type Output = CStr;

    fn index(&self, index: usize) -> &Self::Output {
        unsafe { CStr::from_ptr(self.args[index]) }
    }
}

/**
 * While this handle is held, the player is suspended and the lient can handle the hook.
 * Automatically calls [`mpv_hook_continue`] on drop.
 *
 *  */
pub struct HookHandle<'s> {
    id: u64,
    handle: NonNull<mpv_handle>,
    s: PhantomData<&'s mpv_handle>,
}

impl Debug for HookHandle<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HookHandle").field("id", &self.id).finish()
    }
}

impl Drop for HookHandle<'_> {
    fn drop(&mut self) {
        unsafe { mpv_hook_continue(self.handle.as_ptr(), self.id) };
    }
}

impl MpvEvent<'_> {
    pub(crate) unsafe fn new(
        from: &mpv_event,
        handle: NonNull<mpv_handle>,
    ) -> Result<Option<MpvEvent<'_>>> {
        Ok(Some(match from.event_id {
            mpv_event_id::MPV_EVENT_SHUTDOWN => MpvEvent::Shutdown,
            mpv_event_id::MPV_EVENT_LOG_MESSAGE => {
                let log_message =
                    unsafe { from.data.cast::<mpv_event_log_message>().as_ref_unchecked() };
                MpvEvent::LogMessage {
                    prefix: unsafe { CStr::from_ptr(log_message.prefix) },
                    level: log_message.log_level.into(),
                    level_str: unsafe { CStr::from_ptr(log_message.level) },
                    text: unsafe { CStr::from_ptr(log_message.text) },
                }
            }
            mpv_event_id::MPV_EVENT_GET_PROPERTY_REPLY => {
                get_err_reply(from.error, from.reply_userdata)?;
                let data = unsafe {
                    MpvProperty::new(from.data.cast::<mpv_event_property>().as_ref_unchecked())
                };
                MpvEvent::GetPropertyReply {
                    userdata: from.reply_userdata,
                    data,
                }
            }
            mpv_event_id::MPV_EVENT_SET_PROPERTY_REPLY => {
                get_err_reply(from.error, from.reply_userdata)?;
                MpvEvent::SetPropertyReply {
                    userdata: from.reply_userdata,
                }
            }
            mpv_event_id::MPV_EVENT_COMMAND_REPLY => {
                get_err_reply(from.error, from.reply_userdata)?;
                let data = unsafe { from.data.cast::<MpvNode>().as_ref_unchecked() };
                MpvEvent::CommandReply {
                    userdata: from.reply_userdata,
                    data,
                }
            }
            mpv_event_id::MPV_EVENT_START_FILE => {
                let file = unsafe { from.data.cast::<mpv_event_start_file>().as_ref_unchecked() };
                MpvEvent::StartFile {
                    playlist_entry_id: file.playlist_entry_id,
                }
            }
            mpv_event_id::MPV_EVENT_END_FILE => {
                let end_file = unsafe { from.data.cast::<mpv_event_end_file>().as_ref_unchecked() };
                let reason = match end_file.reason {
                    mpv_end_file_reason::MPV_END_FILE_REASON_EOF => EndFileReason::EOF,
                    mpv_end_file_reason::MPV_END_FILE_REASON_STOP => EndFileReason::Stop,
                    mpv_end_file_reason::MPV_END_FILE_REASON_QUIT => EndFileReason::Quit,
                    mpv_end_file_reason::MPV_END_FILE_REASON_ERROR => {
                        EndFileReason::Error(mpv_error(end_file.error))
                    }
                    mpv_end_file_reason::MPV_END_FILE_REASON_REDIRECT => EndFileReason::Redirect {
                        insert_id: end_file.playlist_insert_id,
                        num: end_file.playlist_insert_num_entries,
                    },
                    _ => EndFileReason::Unknown,
                };
                MpvEvent::EndFile {
                    entry_id: end_file.playlist_entry_id,
                    reason,
                }
            }
            mpv_event_id::MPV_EVENT_FILE_LOADED => MpvEvent::FileLoaded,
            mpv_event_id::MPV_EVENT_CLIENT_MESSAGE => {
                let client_message = unsafe {
                    from.data
                        .cast::<mpv_event_client_message>()
                        .as_ref_unchecked()
                };
                MpvEvent::ClientMessage(ClientMessage {
                    args: unsafe {
                        slice::from_raw_parts(
                            client_message.args,
                            client_message.num_args.try_into().expect("length invalid"),
                        )
                    },
                })
            }
            mpv_event_id::MPV_EVENT_VIDEO_RECONFIG => MpvEvent::VideoReconfig,
            mpv_event_id::MPV_EVENT_AUDIO_RECONFIG => MpvEvent::AudioReconfig,
            mpv_event_id::MPV_EVENT_SEEK => MpvEvent::Seek,
            mpv_event_id::MPV_EVENT_PLAYBACK_RESTART => MpvEvent::PlaybackRestart,
            mpv_event_id::MPV_EVENT_PROPERTY_CHANGE => {
                get_err_reply(from.error, from.reply_userdata)?;
                let data = unsafe {
                    MpvProperty::new(from.data.cast::<mpv_event_property>().as_ref_unchecked())
                };
                MpvEvent::PropertyChange {
                    userdata: from.reply_userdata,
                    data,
                }
            }
            mpv_event_id::MPV_EVENT_QUEUE_OVERFLOW => MpvEvent::QueueOverflow,
            mpv_event_id::MPV_EVENT_HOOK => {
                let hook = unsafe { from.data.cast::<mpv_event_hook>().as_ref_unchecked() };
                MpvEvent::Hook {
                    name: unsafe { CStr::from_ptr(hook.name) },
                    userdata: from.reply_userdata,
                    handle: HookHandle {
                        id: hook.id,
                        handle,
                        s: PhantomData,
                    },
                }
            }
            mpv_event_id::MPV_EVENT_NONE => return Ok(None),
            _ => MpvEvent::Unknown,
        }))
    }
}
