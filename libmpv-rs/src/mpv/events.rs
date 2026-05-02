// Copyright (C) 2016  ParadoxSpiral
//
// This file is part of libmpv-rs.
//
// This library is free software; you can redistribute it and/or
// modify it under the terms of the GNU Lesser General Public
// License as published by the Free Software Foundation; either
// version 2.1 of the License, or (at your option) any later version.
//
// This library is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Lesser General Public License for more details.
//
// You should have received a copy of the GNU Lesser General Public
// License along with this library; if not, write to the Free Software
// Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA  02110-1301  USA

use libmpv_sys::{mpv_event, mpv_event_id as EventId};

#[cfg(feature = "async")]
use crate::mpv::drop_handle::CallbackContext;
use crate::{
    Error, LogLevel, MpvFormat, Result,
    mpv::{
        EndFileReason, Format, Mpv, MpvDropHandle, events, mpv_cstr_to_str, mpv_err, mpv_format,
    },
};

use std::ffi::{CStr, CString};
use std::fmt::Debug;
use std::os::raw as ctype;
use std::ptr::NonNull;
use std::slice;
use std::sync::Arc;

#[cfg(feature = "async")]
use std::{
    ffi::c_void,
    future::{Future, poll_fn},
    sync::atomic::Ordering,
    task::Poll,
};

use super::node::MpvNode;
pub mod mpv_event_id {
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_AUDIO_RECONFIG as AudioReconfig;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_CLIENT_MESSAGE as ClientMessage;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_COMMAND_REPLY as CommandReply;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_END_FILE as EndFile;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_FILE_LOADED as FileLoaded;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_GET_PROPERTY_REPLY as GetPropertyReply;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_HOOK as Hook;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_IDLE as Idle;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_LOG_MESSAGE as LogMessage;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_NONE as None;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_PLAYBACK_RESTART as PlaybackRestart;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_PROPERTY_CHANGE as PropertyChange;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_QUEUE_OVERFLOW as QueueOverflow;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_SEEK as Seek;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_SET_PROPERTY_REPLY as SetPropertyReply;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_SHUTDOWN as Shutdown;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_START_FILE as StartFile;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_TICK as Tick;
    pub use libmpv_sys::mpv_event_id_MPV_EVENT_VIDEO_RECONFIG as VideoReconfig;
}

#[cfg(feature = "async")]
pub struct AsyncContext {
    interval: interval::DefaultInterval,
}

// panics SHALL NOT propagate up from this function
// because this function is extern "C", rust automatically converts panics to aborts
#[cfg(feature = "async")]
#[cfg_attr(feature = "tracing", tracing::instrument(level = "trace"))]
pub(crate) unsafe extern "C" fn wake_callback(cx: *mut c_void) {
    #[cfg(feature = "tracing")]
    {
        tracing::trace!("wake_callback called");
    }
    let context = unsafe { &*cx.cast_const().cast::<CallbackContext>() };
    #[cfg(feature = "tracing")]
    let _enter = context.resource_span.enter();
    #[cfg(feature = "tracing")]
    tracing::trace!(
        target: "runtime::resource::state_update",
        event_ready = true,
        event_ready.op = "override",
    );
    let pin = crossbeam_epoch::pin();
    if let Some(waker) = unsafe {
        context
            .waker
            .load(std::sync::atomic::Ordering::Acquire, &pin)
            .as_ref()
    } {
        waker.wake_by_ref();
    }
}

#[derive(Debug)]
/// Data that is returned by both `GetPropertyReply` and `PropertyChange` events.
pub enum PropertyData<'a> {
    Str(&'a str),
    OsdStr(&'a str),
    Flag(bool),
    Int64(i64),
    Double(ctype::c_double),
    Node(&'a MpvNode),
}

impl PropertyData<'_> {
    // SAFETY: meant to extract the data from an event property. See `mpv_event_property` in
    // `client.h`
    unsafe fn from_raw(format: MpvFormat, ptr: *mut ctype::c_void) -> Result<Self> {
        assert!(!ptr.is_null());
        unsafe {
            match format {
                mpv_format::Flag => Ok(PropertyData::Flag(*ptr.cast::<bool>())),
                mpv_format::String => {
                    let char_ptr = *ptr.cast::<*mut ctype::c_char>();
                    Ok(PropertyData::Str(mpv_cstr_to_str(char_ptr)?))
                }
                mpv_format::OsdString => {
                    let char_ptr = *ptr.cast::<*mut ctype::c_char>();
                    Ok(PropertyData::OsdStr(mpv_cstr_to_str(char_ptr)?))
                }
                mpv_format::Double => Ok(PropertyData::Double(*ptr.cast::<f64>())),
                mpv_format::Int64 => Ok(PropertyData::Int64(*ptr.cast::<i64>())),
                mpv_format::Node => Ok(PropertyData::Node(&*ptr.cast::<MpvNode>())),
                mpv_format::None => unreachable!(),
                _ => unimplemented!(),
            }
        }
    }
}

#[derive(Debug)]
pub enum Event<'a> {
    /// Received when the player is shutting down
    Shutdown,
    /// received when explicitly asked to MPV
    LogMessage {
        prefix: &'a str,
        level: &'a str,
        text: &'a str,
        log_level: LogLevel,
    },
    /// Received when using `get_property_async`
    GetPropertyReply {
        name: &'a str,
        result: PropertyData<'a>,
        reply_userdata: u64,
    },
    /// Received when using `set_property_async`
    SetPropertyReply(u64),
    /// Received when using `command_async`
    CommandReply {
        reply_userdata: u64,
        data: MpvNode,
    },
    /// Event received when a new file is playing
    StartFile {
        playlist_entry_id: i64,
    },
    /// Event received when the file being played currently has stopped, for an error or not
    EndFile(EndFileReason),
    /// Event received when a file has been *loaded*, but has not been started
    FileLoaded,
    ClientMessage(ClientMessage<'a>),
    VideoReconfig,
    AudioReconfig,
    /// The player changed current position
    Seek,
    PlaybackRestart,
    /// Received when used with `observe_property`
    PropertyChange {
        name: &'a str,
        change: PropertyData<'a>,
        reply_userdata: u64,
    },
    /// Received when the Event Queue is full
    QueueOverflow,
    Idle,
    /// A deprecated event
    Deprecated(mpv_event),
}

pub struct EventContextSync {
    drop_handle: Arc<MpvDropHandle>,
    /// The handle to the mpv core
    ctx: NonNull<libmpv_sys::mpv_handle>,
}

unsafe impl Send for EventContextSync {}
unsafe impl Sync for EventContextSync {}

#[cfg(feature = "async")]
pub struct EventContextAsync {
    inner: EventContextSync,
    cx: AsyncContext,
}

pub struct EmptyEventContext;

pub trait EventContextType: sealed::EventContextType {}
impl EventContextType for EmptyEventContext {}
impl EventContextType for EventContextSync {}
#[cfg(feature = "async")]
impl EventContextType for EventContextAsync {}

pub trait EventContext: sealed::EventContext {}

impl EventContext for EventContextSync {}

#[cfg(feature = "async")]
impl EventContext for EventContextAsync {}

#[cfg(feature = "async")]
fn setup_waker(ctx: &MpvDropHandle) -> AsyncContext {
    use crate::events::interval::DefaultInterval;
    unsafe {
        let cx: *const CallbackContext = (&raw const *ctx.callback_cx);
        libmpv_sys::mpv_set_wakeup_callback(
            ctx.ctx.as_ptr(),
            Some(wake_callback),
            cx.cast_mut().cast(),
        );
    };

    #[cfg(feature = "tracing")]
    let _enter = ctx.callback_cx.resource_span.enter();
    AsyncContext {
        interval: <DefaultInterval as interval::Interval>::new(),
    }
}

#[cfg(feature = "async")]
impl EventContextSync {
    #[must_use]
    pub fn enable_async(self) -> EventContextAsync {
        let cx = setup_waker(&self.drop_handle);
        EventContextAsync { inner: self, cx }
    }
}

#[cfg(feature = "async")]
impl Mpv<EventContextSync> {
    #[must_use]
    pub fn enable_async(self) -> Mpv<EventContextAsync> {
        let cx = setup_waker(&self.drop_handle);
        Mpv {
            drop_handle: self.drop_handle,
            ctx: self.ctx,
            event_inline: cx,
        }
    }
}

impl<Event: sealed::EventContext> Mpv<Event> {
    pub fn split_event(self) -> (Mpv<EmptyEventContext>, Event) {
        let new = Mpv {
            drop_handle: self.drop_handle,
            ctx: self.ctx,
            event_inline: (),
        };
        let event = Event::exract(self.event_inline, &new);
        (new, event)
    }
}

impl Mpv<EmptyEventContext> {
    pub fn combine_event<Event: sealed::EventContext + sealed::EventContextExt>(
        self,
        event: Event,
    ) -> Result<Mpv<Event>> {
        if event.get_ctx() == self.ctx {
            Ok(Mpv {
                drop_handle: self.drop_handle,
                ctx: self.ctx,
                event_inline: Event::to_inlined(event),
            })
        } else {
            Err(Error::HandleMismatch)
        }
    }
}

#[derive(Clone, Copy)]
pub struct ClientMessage<'e> {
    args: &'e [*const std::ffi::c_char],
}

impl Debug for ClientMessage<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(*self).finish()
    }
}

unsafe impl Send for ClientMessage<'_> {}
unsafe impl Sync for ClientMessage<'_> {}

impl<'e> IntoIterator for ClientMessage<'e> {
    type Item = &'e CStr;

    type IntoIter = ClientMessageIter<'e>;

    fn into_iter(self) -> Self::IntoIter {
        ClientMessageIter {
            args: self.args.iter(),
        }
    }
}

pub struct ClientMessageIter<'e> {
    args: std::slice::Iter<'e, *const std::ffi::c_char>,
}

unsafe impl Send for ClientMessageIter<'_> {}
unsafe impl Sync for ClientMessageIter<'_> {}

impl<'e> Iterator for ClientMessageIter<'e> {
    type Item = &'e CStr;

    fn next(&mut self) -> Option<Self::Item> {
        self.args.next().map(|s| unsafe { CStr::from_ptr(*s) })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.args.size_hint()
    }
}
impl DoubleEndedIterator for ClientMessageIter<'_> {
    fn next_back(&mut self) -> Option<Self::Item> {
        self.args.next_back().map(|s| unsafe { CStr::from_ptr(*s) })
    }
}
impl ExactSizeIterator for ClientMessageIter<'_> {}

pub trait EventContextExt: sealed::EventContextExt {
    /// Enable an event.
    fn enable_event(&self, ev: events::EventId) -> Result<()> {
        mpv_err((), unsafe {
            libmpv_sys::mpv_request_event(self.get_ctx().as_ptr(), ev, 1)
        })
    }

    /// Enable all, except deprecated, events.
    fn enable_all_events(&self) -> Result<()> {
        for i in (2..9).chain(16..19).chain(20..23).chain(24..26) {
            self.enable_event(i)?;
        }
        Ok(())
    }

    /// Disable an event.
    fn disable_event(&self, ev: events::EventId) -> Result<()> {
        mpv_err((), unsafe {
            libmpv_sys::mpv_request_event(self.get_ctx().as_ptr(), ev, 0)
        })
    }

    /// Diable all deprecated events.
    fn disable_deprecated_events(&self) -> Result<()> {
        self.disable_event(libmpv_sys::mpv_event_id_MPV_EVENT_IDLE)?;
        Ok(())
    }

    /// Diable all events.
    fn disable_all_events(&self) -> Result<()> {
        for i in 2u32..26 {
            self.disable_event(i)?;
        }
        Ok(())
    }

    /// Observe `name` property for changes. `id` can be used to unobserve this (or many) properties
    /// again.
    fn observe_property(&self, name: &str, format: Format, id: u64) -> Result<()> {
        let name = CString::new(name)?;
        mpv_err((), unsafe {
            libmpv_sys::mpv_observe_property(
                self.get_ctx().as_ptr(),
                id,
                name.as_ptr(),
                format.as_mpv_format(),
            )
        })
    }

    /// Unobserve any property associated with `id`.
    fn unobserve_property(&self, id: u64) -> Result<()> {
        mpv_err((), unsafe {
            libmpv_sys::mpv_unobserve_property(self.get_ctx().as_ptr(), id)
        })
    }

    /// Wait for `timeout` seconds for an `Event`. Passing `0` as `timeout` will poll.
    /// For more information, as always, see the mpv-sys docs of `mpv_wait_event`.
    ///
    /// This function is intended to be called repeatedly in a wait-event loop.
    ///
    /// Returns `Some(Err(...))` if there was invalid utf-8, or if either an
    /// `MPV_EVENT_GET_PROPERTY_REPLY`, `MPV_EVENT_SET_PROPERTY_REPLY`, `MPV_EVENT_COMMAND_REPLY`,
    /// or `MPV_EVENT_PROPERTY_CHANGE` event failed, or if `MPV_EVENT_END_FILE` reported an error.
    fn wait_event(&mut self, timeout: f64) -> Option<Result<Event<'_>>> {
        unsafe { self.wait_event_unsafe(timeout) }
    }

    /// unsafe, internal version of `wait_event`.
    /// # Safety
    /// requires that all previously returned Events are now unreachable.
    unsafe fn wait_event_unsafe(&self, timeout: f64) -> Option<Result<Event<'static>>> {
        let event = unsafe { *libmpv_sys::mpv_wait_event(self.get_ctx().as_ptr(), timeout) };
        if event.event_id != mpv_event_id::None
            && let Err(e) = mpv_err((), event.error)
        {
            return Some(Err(e));
        }

        match event.event_id {
            mpv_event_id::None => None,
            mpv_event_id::Shutdown => Some(Ok(Event::Shutdown)),
            mpv_event_id::LogMessage => {
                let log_message =
                    unsafe { *event.data.cast::<libmpv_sys::mpv_event_log_message>() };

                let prefix = unsafe { mpv_cstr_to_str(log_message.prefix) };
                Some(prefix.and_then(|prefix| {
                    Ok(Event::LogMessage {
                        prefix,
                        level: unsafe { mpv_cstr_to_str(log_message.level)? },
                        text: unsafe { mpv_cstr_to_str(log_message.text)? },
                        log_level: log_message.log_level,
                    })
                }))
            }
            mpv_event_id::GetPropertyReply => {
                let property = unsafe { *event.data.cast::<libmpv_sys::mpv_event_property>() };

                let name = unsafe { mpv_cstr_to_str(property.name) };
                Some(name.and_then(|name| {
                    // SAFETY: safe because we are passing format + data from an mpv_event_property
                    let result = unsafe { PropertyData::from_raw(property.format, property.data) }?;

                    Ok(Event::GetPropertyReply {
                        name,
                        result,
                        reply_userdata: event.reply_userdata,
                    })
                }))
            }
            mpv_event_id::SetPropertyReply => Some(mpv_err(
                Event::SetPropertyReply(event.reply_userdata),
                event.error,
            )),
            mpv_event_id::CommandReply => {
                if event.error < 0 {
                    Some(Err(event.error.into()))
                } else {
                    let data = unsafe { &*(event.data.cast::<libmpv_sys::mpv_event_command>()) };
                    Some(Ok(Event::CommandReply {
                        reply_userdata: event.reply_userdata,
                        data: unsafe { MpvNode::new(data.result) },
                    }))
                }
            }
            mpv_event_id::StartFile => {
                let start_file = unsafe { *event.data.cast::<libmpv_sys::mpv_event_start_file>() };
                Some(Ok(Event::StartFile {
                    playlist_entry_id: start_file.playlist_entry_id,
                }))
            }
            mpv_event_id::EndFile => {
                let end_file = unsafe { *event.data.cast::<libmpv_sys::mpv_event_end_file>() };

                if let Err(e) = mpv_err((), end_file.error) {
                    Some(Err(e))
                } else {
                    Some(Ok(Event::EndFile(end_file.reason as _)))
                }
            }
            mpv_event_id::FileLoaded => Some(Ok(Event::FileLoaded)),
            mpv_event_id::ClientMessage => {
                let client_message =
                    unsafe { *event.data.cast::<libmpv_sys::mpv_event_client_message>() };

                match usize::try_from(client_message.num_args) {
                    Ok(num_args) => Some(Ok(Event::ClientMessage(ClientMessage {
                        args: unsafe { slice::from_raw_parts(client_message.args, num_args) },
                    }))),
                    Err(e) => Some(Err(e.into())),
                }
            }
            mpv_event_id::VideoReconfig => Some(Ok(Event::VideoReconfig)),
            mpv_event_id::AudioReconfig => Some(Ok(Event::AudioReconfig)),
            mpv_event_id::Seek => Some(Ok(Event::Seek)),
            mpv_event_id::PlaybackRestart => Some(Ok(Event::PlaybackRestart)),
            mpv_event_id::PropertyChange => {
                let property = unsafe { *event.data.cast::<libmpv_sys::mpv_event_property>() };

                // This happens if the property is not available. For example,
                // if you reached EndFile while observing a property.
                if property.format == mpv_format::None {
                    None
                } else {
                    let name = unsafe { mpv_cstr_to_str(property.name) };
                    Some(name.and_then(|name| {
                        // SAFETY: safe because we are passing format + data from an mpv_event_property
                        let change =
                            unsafe { PropertyData::from_raw(property.format, property.data) }?;

                        Ok(Event::PropertyChange {
                            name,
                            change,
                            reply_userdata: event.reply_userdata,
                        })
                    }))
                }
            }
            mpv_event_id::QueueOverflow => Some(Ok(Event::QueueOverflow)),
            mpv_event_id::Idle => Some(Ok(Event::Idle)),
            _ => Some(Ok(Event::Deprecated(event))),
        }
    }
}

impl<T: sealed::EventContextExt> EventContextExt for T {}

#[cfg(feature = "async")]
pub trait EventContextAsyncExt:
    sealed::EventContextAsyncExt + EventContextExt + Send + Sync
{
    fn wait_event_async(&mut self) -> impl Future<Output = Result<Event<'_>>> + Send + Sync {
        poll_fn(move |cx| unsafe { self.poll_wait_event_inner(cx) })
    }
    /**
    # Safety
    ensure that 's is properly bound. It must be impossible to call any `wait_event` while the last returned event is still alive.
     */
    unsafe fn poll_wait_event_inner<'s>(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<Event<'s>>> {
        let callback_cx = self.get_callback_cx();
        #[cfg(feature = "tracing")]
        let enter = callback_cx.resource_span.enter();
        if let Some(v) = unsafe { self.wait_event_unsafe(0.0) } {
            return Poll::Ready(v);
        }
        {
            let pin = crossbeam_epoch::pin();
            let waker = cx.waker();
            if let Some(current) =
                unsafe { callback_cx.waker.load(Ordering::Acquire, &pin).as_ref() }
                && current.will_wake(waker)
            {
            } else {
                callback_cx.waker.store(
                    crossbeam_epoch::Owned::new(waker.clone()),
                    Ordering::Release,
                );
            }
        }
        if let Some(v) = unsafe { self.wait_event_unsafe(0.0) } {
            return Poll::Ready(v);
        }
        #[cfg(feature = "tracing")]
        tracing::trace!(
            target: "runtime::resource::state_update",
            event_ready = false,
            event_ready.op = "override",
        );
        #[cfg(feature = "tracing")]
        drop(enter);
        //mpv doesn't run the callback on destruction, poll regularly anyway to avoid deadlocks
        interval::Interval::poll(&mut self.get_async_cx().interval, cx);
        Poll::Pending
    }
    fn poll_wait_event<'s>(
        &'s mut self,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Result<Event<'s>>> {
        unsafe { self.poll_wait_event_inner(cx) }
    }
}

#[cfg(feature = "async")]
impl<T: sealed::EventContextAsyncExt + EventContextExt + Send + Sync> EventContextAsyncExt for T {}

mod sealed {
    use std::ptr::NonNull;

    #[cfg(feature = "async")]
    use super::{AsyncContext, EventContextAsync};
    use super::{EmptyEventContext, EventContextSync, Mpv};

    pub trait EventContextType {
        type Inlined: Send + Sync;
    }
    impl EventContextType for EventContextSync {
        type Inlined = ();
    }
    #[cfg(feature = "async")]
    impl EventContextType for EventContextAsync {
        type Inlined = AsyncContext;
    }
    impl EventContextType for EmptyEventContext {
        type Inlined = ();
    }

    pub trait EventContext: super::EventContextType {
        fn exract(inline: Self::Inlined, cx: &Mpv<EmptyEventContext>) -> Self;
        fn to_inlined(self) -> Self::Inlined;
    }

    impl EventContext for EventContextSync {
        fn exract(_inline: Self::Inlined, cx: &Mpv<EmptyEventContext>) -> Self {
            Self {
                ctx: cx.ctx,
                drop_handle: cx.drop_handle.clone(),
            }
        }
        fn to_inlined(self) -> Self::Inlined {}
    }

    #[cfg(feature = "async")]
    impl EventContext for EventContextAsync {
        fn exract(inline: Self::Inlined, cx: &Mpv<EmptyEventContext>) -> Self {
            Self {
                inner: EventContextSync::exract((), cx),
                cx: inline,
            }
        }

        fn to_inlined(self) -> Self::Inlined {
            self.cx
        }
    }

    /// # Safety
    /// ctx must be valid
    pub unsafe trait EventContextExt {
        ///this must return a valid handle
        fn get_ctx(&self) -> NonNull<libmpv_sys::mpv_handle>;
    }
    unsafe impl EventContextExt for EventContextSync {
        fn get_ctx(&self) -> NonNull<libmpv_sys::mpv_handle> {
            self.ctx
        }
    }

    #[cfg(feature = "async")]
    unsafe impl EventContextExt for EventContextAsync {
        fn get_ctx(&self) -> NonNull<libmpv_sys::mpv_handle> {
            self.inner.ctx
        }
    }

    unsafe impl EventContextExt for Mpv<EventContextSync> {
        fn get_ctx(&self) -> NonNull<libmpv_sys::mpv_handle> {
            self.ctx
        }
    }

    #[cfg(feature = "async")]
    unsafe impl EventContextExt for Mpv<EventContextAsync> {
        fn get_ctx(&self) -> NonNull<libmpv_sys::mpv_handle> {
            self.ctx
        }
    }
    #[cfg(feature = "async")]
    pub trait EventContextAsyncExt: EventContextExt {
        fn get_callback_cx(&self) -> &crate::mpv::drop_handle::CallbackContext;
        fn get_async_cx(&mut self) -> &mut AsyncContext;
    }
    #[cfg(feature = "async")]
    impl EventContextAsyncExt for EventContextAsync {
        fn get_callback_cx(&self) -> &crate::mpv::drop_handle::CallbackContext {
            &self.inner.drop_handle.callback_cx
        }

        fn get_async_cx(&mut self) -> &mut AsyncContext {
            &mut self.cx
        }
    }
    #[cfg(feature = "async")]
    impl EventContextAsyncExt for Mpv<EventContextAsync> {
        fn get_callback_cx(&self) -> &crate::mpv::drop_handle::CallbackContext {
            &self.drop_handle.callback_cx
        }

        fn get_async_cx(&mut self) -> &mut AsyncContext {
            &mut self.event_inline
        }
    }
}

#[cfg(feature = "async")]
mod interval {
    use std::{task::Context, time::Duration};

    pub trait Interval {
        fn new() -> Self;
        fn poll(&mut self, cx: &mut Context);
    }

    #[cfg(feature = "tokio")]
    impl Interval for tokio::time::Interval {
        fn new() -> Self {
            let mut interval = tokio::time::interval(Duration::from_millis(500));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            interval
        }

        fn poll(&mut self, cx: &mut Context) {
            while self.poll_tick(cx).is_ready() {}
        }
    }

    #[cfg(feature = "tokio")]
    pub type DefaultInterval = tokio::time::Interval;

    #[cfg(not(any(feature = "tokio")))]
    compile_error!("some async runtime must be enabled");
}
