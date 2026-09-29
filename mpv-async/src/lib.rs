/// Event related types
pub mod events;
mod helper;
pub use helper::MpvExt;
/// handling of mpv nodes
pub mod nodes;
/// A wrapper that turns [Mpv] into a Stream
#[cfg(feature = "stream")]
pub mod stream;
#[cfg(feature = "valuable")]
mod valuable;

use std::{
    cell::UnsafeCell,
    ffi::{CStr, c_int, c_uint, c_void},
    fmt::{Debug, Display},
    mem::{self, ManuallyDrop},
    ptr::{self, NonNull, null},
    sync::atomic::{AtomicBool, Ordering::SeqCst},
    task::{Context, Poll, Waker},
    time::Instant,
};

use arcshift::ArcShift;
use mpv_sys::{
    ClientApiVersion, mpv_client_name, mpv_command_node, mpv_command_node_async, mpv_create,
    mpv_create_client, mpv_create_weak_client, mpv_del_property, mpv_error, mpv_get_property,
    mpv_get_property_async, mpv_handle, mpv_hook_add, mpv_load_config_file, mpv_node,
    mpv_observe_property, mpv_request_log_messages, mpv_set_property, mpv_set_property_async,
    mpv_set_wakeup_callback, mpv_terminate_destroy, mpv_unobserve_property, mpv_wait_event,
};

#[cfg(feature = "macros")]
#[doc(hidden)]
pub use mpv_async_macros as macros;

#[cfg(feature = "stream")]
pub use crate::stream::EventStream;

use crate::{
    events::MpvEvent,
    nodes::{
        FromFormat, MpvFormat, MpvNodeList, MpvNodeMap, MpvOwnedNode, MpvStackNode, ToFormat,
        ToMpvNode,
    },
};

/// check that the version of the linked library is compatible to the generated bindings.
pub fn check_mpv_version() -> Result<()> {
    let header = ClientApiVersion::HEADER;
    let lib = ClientApiVersion::linked();
    #[cfg(feature = "tracing")]
    tracing::info!("Detected mpv client api version {lib}. Compiled against {header}");
    'err: {
        if header.major != lib.major {
            #[cfg(feature = "tracing")]
            tracing::error!(
                header = header.major,
                library = lib.major,
                "Mpv cliant api major version mismatch detected"
            );
            break 'err;
        }
        if header.minor > lib.minor {
            #[cfg(feature = "tracing")]
            tracing::error!(
                header = header.minor,
                library = lib.minor,
                "Mpv client api minor version mismatch detected"
            );
            break 'err;
        }
        return Ok(());
    };
    Err(Error::new(mpv_error::MPV_ERROR_UNSUPPORTED.into()))
}

unsafe extern "C" fn wakeup_callback(d: *mut c_void) {
    //TODO investigate multi threading in mpv
    let context = unsafe { d.cast::<CallbackContext>().as_ref_unchecked() };
    if context.should_wake.swap(false, SeqCst)
        && let Some(waker) = unsafe { context.waker.get().as_mut_unchecked() }.get()
    {
        waker.wake_by_ref();
    }
}

mod mpv_state {
    use crate::{Initialized, Initializing};

    pub trait StateSealed {}
    impl StateSealed for Initialized {}
    impl StateSealed for Initializing {}
}

/// Marker for the current state of mpv
pub trait State: mpv_state::StateSealed {}
/// Mpv is initializing
///
/// At this stage you can set options some of which can not be changed while the player is running.
pub struct Initializing;
impl State for Initializing {}

struct CallbackContext {
    should_wake: AtomicBool,
    waker: UnsafeCell<ArcShift<Option<Waker>>>,
}
/// Mpv is initialized and ready for playback
///
/// The full range of the mpv client api is now available. You can modify the playlist, adjust and read all properties and wait for events.
pub struct Initialized {
    waker_set: ArcShift<Option<Waker>>,
    callback_storage: Box<CallbackContext>,
}
impl State for Initialized {}
unsafe impl Sync for Initialized {}

/// Owned mpv client handle
///
/// Handles are reference counted and once all handles have been destroyed, the player is stopped and all state is destructed.
///
/// After initialization, you can clone this handle with [`Mpv::clone_client()`] to create additional client handels that have each independent event queues and observed properties.
/// If you use [`Mpv::clone_client_weak()`] you get a weak handle that does not add to the reference count.
pub struct Mpv<S: State = Initialized> {
    handle: NonNull<mpv_handle>,
    state: S,
    #[cfg(feature = "tracing")]
    spans: Spans,
}

#[cfg(feature = "tracing")]
#[derive(Clone)]
struct Spans {
    resource_span: tracing::Span,
    async_op: tracing::Span,
    poll: tracing::Span,
}

#[cfg(feature = "tracing")]
#[track_caller]
fn make_spans(weak: bool) -> Spans {
    let location = std::panic::Location::caller();
    let resource = tracing::trace_span!(
        parent: None,
        "runtime.resource",
        concrete_type = "Mpv",
        kind = "media",
        loc.file = location.file(),
        loc.line = location.line(),
        loc.col = location.column(),
    );
    let span = resource.entered();
    tracing::trace!(
        target: "runtime::resource::state_update",
        weak,
        weak.op = "override"
    );
    tracing::trace!(
        target: "runtime::resource::state_update",
        waker_set = false,
        waker_set.op = "override",
    );
    tracing::trace!(
        target: "runtime::resource::state_update",
        events = 0usize,
        events.op = "override",
    );
    tracing::trace!(
        target: "runtime::resource::state_update",
        pending_poll = 0usize,
        pending_poll.op = "override",
    );
    let async_op = tracing::trace_span!("runtime.resource.async_op", source = "Mpv::wait_event");
    let poll = async_op.in_scope(|| tracing::trace_span!("runtime.resource.async_op.poll"));
    Spans {
        poll,
        async_op,
        resource_span: span.exit(),
    }
}

impl<S: State> Mpv<S> {
    /// Set property `name` to `data`
    ///
    /// Depending on the current [`State`] `S` this either sets the [property](https://mpv.io/manual/stable/#property-list) (with state [`Initialized`])
    /// or the [option](https://mpv.io/manual/stable/#options) (with state [`Initializing`]).
    pub fn set_property<F: ToFormat>(&self, name: &CStr, data: F) -> Result<()> {
        get_err(data.do_with(|data| unsafe {
            mpv_set_property(
                self.handle.as_ptr(),
                name.as_ptr(),
                F::FORMAT,
                data.cast_mut(),
            )
        }))
    }

    /// Run [input command](https://mpv.io/manual/stable/#list-of-input-commands)
    ///
    /// `args` must be either a [`MpvNodeList`] or [`MpvNodeMap`]
    pub fn command(&self, args: impl MpvCommandArg) -> Result<MpvOwnedNode> {
        let mut res: mpv_node = unsafe { mem::zeroed() };
        get_err(args.with(|p| unsafe {
            mpv_command_node(self.handle.as_ptr(), p.cast_mut(), &raw mut res)
        }))?;
        Ok(unsafe { MpvOwnedNode::new(res) })
    }

    /// The name of the client.
    ///
    /// Mainly usefull for use with the [script-message-to](https://mpv.io/manual/stable/#command-interface-script-message-to[-]]]) command.
    /// You can for example use [keybind](https://mpv.io/manual/stable/#command-interface-keybind-%3Cname%3E-%3Ccmd%3E-[%3Ccomment%3E])
    /// together with [script-message-to](https://mpv.io/manual/stable/#command-interface-script-message-to[-]]]) to have the client handle
    /// key presses on the player window.
    #[must_use]
    pub fn client_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(mpv_client_name(self.handle.as_ptr())) }
    }

    /// Request log messages
    ///
    /// Mpv will send messages at or above the specified level to the client as [`LogMessage`](MpvEvent::LogMessage).
    pub fn request_log_messages(&self, min_level: &CStr) -> Result<()> {
        get_err(unsafe { mpv_request_log_messages(self.handle.as_ptr(), min_level.as_ptr()) })
    }

    /// Drop this client and terminate the player.
    ///
    /// Terminates the player even if other client handles still exist.
    /// All other clients will receive the [`MpvEvent::Shutdown`] event.
    pub fn terminate(self) {
        let mut this = ManuallyDrop::new(self);
        unsafe {
            mpv_terminate_destroy(this.handle.as_ptr());
            ptr::drop_in_place(&raw mut this.state);
        }
    }
}

impl Mpv<Initializing> {
    /**
     * Load a config file.
     *
     * This loads and parses the file, and sets every entry in the config file's default section.
     * The filename should be an absolute path. If it isn't, the actual path used\n is unspecified.
     *  */
    pub fn load_config_file(&self, filename: &CStr) -> Result<()> {
        get_err(unsafe { mpv_load_config_file(self.handle.as_ptr(), filename.as_ptr()) })
    }

    /**
     * Initialize an uninitialized mpv instance.
     *
     * Only the following options are required to be set _before_ `mpv_initialize()`:
     * - options which are only read at initialization time:
     *   - config
     *   - config-dir
     *   - input-conf
     *   - load-scripts
     *   - script
     *   - player-operation-mode
     *   - input-app-events (macOS)
     * - all encoding mode options
     *  */
    pub fn initialize(self) -> Result<Mpv> {
        get_err(unsafe { mpv_sys::mpv_initialize(self.handle.as_ptr()) })?;

        let handle = self.handle;
        let spans = self.spans.clone();
        let state = add_callback(handle);
        mem::forget(self);
        Ok(Mpv {
            handle,
            state,
            #[cfg(feature = "tracing")]
            spans,
        })
    }
}

fn add_callback(handle: NonNull<mpv_handle>) -> Initialized {
    let waker_set = ArcShift::new(None);
    let callback_storage = Box::new(CallbackContext {
        should_wake: AtomicBool::new(false),
        waker: UnsafeCell::new(waker_set.clone()),
    });
    unsafe {
        mpv_set_wakeup_callback(
            handle.as_ptr(),
            Some(wakeup_callback),
            Box::as_ptr(&callback_storage).cast_mut().cast(),
        );
    };
    Initialized {
        waker_set,
        callback_storage,
    }
}

impl Mpv {
    /// Create new mpv player with one connected client handle.
    ///
    /// you can finish intialization with [`Mpv::initialize()`].
    pub fn new() -> Result<Mpv<Initializing>> {
        check_mpv_version()?;
        let handle = unsafe { NonNull::new(mpv_create()) }
            .ok_or_else(|| Error::new(mpv_error::MPV_ERROR_GENERIC.into()))?;
        Ok(Mpv {
            handle,
            state: Initializing,
            #[cfg(feature = "tracing")]
            spans: make_spans(false),
        })
    }

    /// Get property `name`
    ///
    /// The used format is determined from the return type.
    /// [Consult the mpv documentation for the list of available properties.](https://mpv.io/manual/stable/#property-list)
    pub fn get_property<F: FromFormat>(&self, name: &CStr) -> Result<F> {
        let mut receiver: F::MpvType = unsafe { mem::zeroed() };
        get_err(unsafe {
            mpv_get_property(
                self.handle.as_ptr(),
                name.as_ptr(),
                F::FORMAT,
                (&raw mut receiver).cast(),
            )
        })?;
        Ok(F::finanlize(receiver))
    }

    /// Get property `name` as event
    ///
    /// The curent value of the property will be returned throuh an event.
    /// [Consult the mpv documentation for the list of available properties.](https://mpv.io/manual/stable/#property-list)
    pub fn get_property_async(
        &self,
        name: &CStr,
        format: MpvFormat,
        reply_userdata: u64,
    ) -> Result<()> {
        get_err(unsafe {
            mpv_get_property_async(
                self.handle.as_ptr(),
                reply_userdata,
                name.as_ptr(),
                format.ffi(),
            )
        })
    }

    /// Set property `name` to `data`
    ///
    /// The property is not changed immediately. Once the property is modified, the result (success or failure) is returned as an event.
    pub fn set_property_async<F: ToFormat>(
        &self,
        name: &CStr,
        data: F,
        reply_userdata: u64,
    ) -> Result<()> {
        get_err(data.do_with(|p| unsafe {
            mpv_set_property_async(
                self.handle.as_ptr(),
                reply_userdata,
                name.as_ptr(),
                F::FORMAT,
                p.cast_mut(),
            )
        }))
    }
    /// Delete property `name`
    pub fn del_property(&self, name: &CStr) -> Result<()> {
        get_err(unsafe { mpv_del_property(self.handle.as_ptr(), name.as_ptr()) })
    }

    /// Get notified on changes to property `name`
    ///
    /// Modified properties are only reported once the event queue is empty and multiple updates may be coalesced.
    #[inline]
    pub fn observe_property(
        &self,
        reply_userdata: u64,
        name: &CStr,
        format: MpvFormat,
    ) -> Result<()> {
        get_err(unsafe {
            mpv_observe_property(
                self.handle.as_ptr(),
                reply_userdata,
                name.as_ptr(),
                format.ffi(),
            )
        })
    }

    /// Stop observing property registered with `reply_userdata`
    pub fn unobserve_property(&self, reply_userdata: u64) -> Result<c_uint> {
        let res = unsafe { mpv_unobserve_property(self.handle.as_ptr(), reply_userdata) };
        c_uint::try_from(res).map_err(|_| Error::new(mpv_error(res).into()))
    }

    /// Create a new client handle connected to the same player
    ///
    /// If `name` is set to `None`, mpv selects some unspecified client name.
    /// The new client has a seperate set of observed properties and a seperate event queue.
    /// That means that while all clients receive general events like [`StartFile`](MpvEvent::StartFile) (if enabled),
    /// they only receive replies to there own async requests and observed properties.
    /// The set of requested events and observed properties is reset.
    pub fn clone_client(&self, name: Option<&CStr>) -> Result<Self> {
        let name = name.map_or_else(null, CStr::as_ptr);
        let handle = NonNull::new(unsafe { mpv_create_client(self.handle.as_ptr(), name) })
            .ok_or_else(|| Error::new(mpv_error::MPV_ERROR_GENERIC.into()))?;
        Ok(Self {
            handle,
            state: add_callback(handle),
            #[cfg(feature = "tracing")]
            spans: make_spans(false),
        })
    }
    /// Create new weak client handle
    ///
    /// Behaves like [`clone_client()`](Mpv::clone_client()), but the returned handle will not keep the player alive.
    /// Once all non weak handles are destroyed, the player will ex
    pub fn clone_client_weak(&self, name: Option<&CStr>) -> Result<Self> {
        let name = name.map_or_else(null, CStr::as_ptr);
        let handle = NonNull::new(unsafe { mpv_create_weak_client(self.handle.as_ptr(), name) })
            .ok_or_else(|| Error::new(mpv_error::MPV_ERROR_GENERIC.into()))?;
        Ok(Self {
            handle,
            state: add_callback(handle),
            #[cfg(feature = "tracing")]
            spans: make_spans(true),
        })
    }

    ///  Run [input command](https://mpv.io/manual/stable/#list-of-input-commands) async
    ///
    ///  The result of the operation is returned through the [`CommandReply`](MpvEvent::CommandReply) event.
    pub fn command_async(&self, args: &MpvNodeList, reply_userdata: u64) -> Result<()> {
        get_err(args.with(|p| unsafe {
            mpv_command_node_async(self.handle.as_ptr(), reply_userdata, p.cast_mut())
        }))
    }

    /// register some hook with this client handle
    ///
    /// It is generally recommended to nor use this api if possible, because the player is blocked until the hook is handled.
    /// Hooks are invoked through the [Hook](MpvEvent::Hook) event. For more detauils see the [mpv documentation](https://mpv.io/manual/stable/#hooks)
    ///
    pub fn add_hook(&self, reply_userdata: u64, name: &CStr, priority: c_int) -> Result<()> {
        get_err(unsafe {
            mpv_hook_add(
                self.handle.as_ptr(),
                reply_userdata,
                name.as_ptr(),
                priority,
            )
        })
    }

    /// Reveive event.
    ///
    /// waits indefinitely for the next event.
    pub fn wait_event(&mut self) -> Result<MpvEvent<'_>> {
        #[cfg(feature = "tracing")]
        let _entered = self.spans.resource_span.enter();
        loop {
            if let Some(res) = unsafe { unsafe_wait_event(self.handle, -1.0) }? {
                break Ok(res);
            }
        }
    }

    /// Reveive event or timeout.
    ///
    /// Will wait until `until`.
    pub fn wait_event_timeout_until(&mut self, until: Instant) -> Result<Option<MpvEvent<'_>>> {
        #[cfg(feature = "tracing")]
        let _entered = self.spans.resource_span.enter();
        loop {
            let Some(time) = until.checked_duration_since(Instant::now()) else {
                break Ok(None);
            };
            if let Some(res) = unsafe { unsafe_wait_event(self.handle, time.as_secs_f64()) }? {
                break Ok(Some(res));
            }
        }
    }
    unsafe fn unsafe_poll_wait_event<'s>(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<MpvEvent<'s>>> {
        #[cfg(feature = "tracing")]
        let _entered = self.spans.resource_span.enter();
        #[cfg(feature = "tracing")]
        let _entered = self.spans.async_op.enter();
        #[cfg(feature = "tracing")]
        let _entered = self.spans.poll.enter();
        'pending: {
            let res = match unsafe { unsafe_wait_event(self.handle, 0.0) } {
                Err(e) => Err(e),
                Ok(None) => {
                    break 'pending;
                }
                Ok(Some(v)) => Ok(v),
            };
            #[cfg(feature = "tracing")]
            tracing::trace!(
                target: "runtime::resource::poll_op",
                op_name = "poll_wait_event",
                is_ready = true,
            );
            return Poll::Ready(res);
        }
        if self.state.waker_set.rcu_maybe(|prev| {
            let waker = cx.waker();
            if let Some(prev) = prev
                && prev.will_wake(waker)
            {
                None
            } else {
                Some(Some(waker.clone()))
            }
        }) {
            #[cfg(feature = "tracing")]
            tracing::trace!(
                target: "runtime::resource::state_update",
                waker_set = true,
                waker_set.op = "override",
            );
        }
        self.state.callback_storage.should_wake.store(true, SeqCst);
        let res = match unsafe { unsafe_wait_event(self.handle, 0.0) } {
            Err(e) => Err(e),
            Ok(None) => {
                #[cfg(feature = "tracing")]
                tracing::trace!(
                    target: "runtime::resource::poll_op",
                    op_name = "poll_wait_event",
                    is_ready = false,
                );
                #[cfg(feature = "tracing")]
                tracing::trace!(
                    target: "runtime::resource::state_update",
                    pending_poll = 1usize,
                    pending_poll.op = "add",
                );
                return Poll::Pending;
            }
            Ok(Some(v)) => Ok(v),
        };
        #[cfg(feature = "tracing")]
        tracing::trace!(
            target: "runtime::resource::poll_op",
            op_name = "poll_wait_event",
            is_ready = true,
        );
        Poll::Ready(res)
    }

    /// Poll for next event
    ///
    /// Will register the current task for wakeup. Only the last caller will be notified.
    pub fn poll_wait_event<'s>(&'s mut self, cx: &mut Context<'_>) -> Poll<Result<MpvEvent<'s>>> {
        unsafe { self.unsafe_poll_wait_event(cx) }
    }

    /// Get next event(async version)
    pub const fn wait_event_async(&mut self) -> WaitEvent<'_> {
        WaitEvent { client: self }
    }

    /// Create a stram over mpv events mapped by `mapper`.
    #[cfg(feature = "stream")]
    pub const fn event_stream<T, F: FnMut(Result<MpvEvent<'_>>) -> T>(
        &mut self,
        mapper: F,
    ) -> EventStream<'_, T, F> {
        EventStream {
            client: self,
            mapper,
            exit: false,
        }
    }
}

unsafe fn unsafe_wait_event<'s>(
    handle: NonNull<mpv_handle>,
    timeout: f64,
) -> Result<Option<MpvEvent<'s>>> {
    let event = unsafe { mpv_wait_event(handle.as_ptr(), timeout).as_ref_unchecked() };
    unsafe { MpvEvent::new(event, handle) }
}

impl<S: State> Drop for Mpv<S> {
    fn drop(&mut self) {
        unsafe { mpv_sys::mpv_destroy(self.handle.as_ptr()) };
    }
}

/// Future for getting the next event
pub struct WaitEvent<'s> {
    client: &'s mut Mpv,
}

impl<'s> Future for WaitEvent<'s> {
    type Output = Result<MpvEvent<'s>>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { self.get_mut().client.unsafe_poll_wait_event(cx) }
    }
}

/// Error returned by mpv
///
/// can optionally have a `reply_userdata` if originating from an event
pub struct Error {
    pub error: MpvError,
    pub reply_userdata: Option<u64>,
}

impl Error {
    #[must_use]
    pub const fn new(error: MpvError) -> Self {
        Self {
            error,
            reply_userdata: None,
        }
    }
    #[must_use]
    pub const fn new_reply(error: MpvError, userdata: u64) -> Self {
        Self {
            error,
            reply_userdata: Some(userdata),
        }
    }
}

#[repr(transparent)]
#[derive(Clone, Copy)]
pub struct MpvError(pub mpv_error);
impl MpvError {
    #[must_use]
    pub fn get_error_string(self) -> &'static CStr {
        unsafe { CStr::from_ptr(mpv_sys::mpv_error_string(self.0.0)) }
    }
    #[must_use]
    pub const fn new(inner: mpv_error) -> Self {
        Self(inner)
    }
}

impl From<mpv_error> for MpvError {
    fn from(value: mpv_error) -> Self {
        Self(value)
    }
}

impl Debug for MpvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self.get_error_string(), f)
    }
}

impl Display for MpvError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.get_error_string().to_str().unwrap_or("invalid utf-8"))
    }
}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(userdata) = self.reply_userdata {
            f.write_str("[reply_userdata=")?;
            Display::fmt(&userdata, f)?;
            f.write_str("] ")?;
        }
        Display::fmt(&self.error, f)
    }
}

impl Debug for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(self, f)
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

fn get_err(v: c_int) -> Result<()> {
    let v = mpv_error(v);
    if v < mpv_error::MPV_ERROR_SUCCESS {
        Err(Error::new(v.into()))
    } else {
        Ok(())
    }
}
fn get_err_reply(v: c_int, reply: u64) -> Result<()> {
    let v = mpv_error(v);
    if v < mpv_error::MPV_ERROR_SUCCESS {
        Err(Error::new_reply(v.into(), reply))
    } else {
        Ok(())
    }
}

pub trait MpvCommandArg {
    fn with<T>(self, f: impl FnOnce(*const mpv_node) -> T) -> T;
}

impl MpvCommandArg for &MpvNodeMap<'_> {
    fn with<T>(self, f: impl FnOnce(*const mpv_node) -> T) -> T {
        let args = self.node();
        f(args.inner())
    }
}
impl MpvCommandArg for &MpvNodeList<'_> {
    fn with<T>(self, f: impl FnOnce(*const mpv_node) -> T) -> T {
        let args = self.node();
        f(args.inner())
    }
}
impl MpvCommandArg for &[MpvStackNode<'_>] {
    fn with<T>(self, f: impl FnOnce(*const mpv_node) -> T) -> T {
        let args = MpvNodeList::new(self);
        args.with(f)
    }
}
impl<const N: usize> MpvCommandArg for &[MpvStackNode<'_>; N] {
    fn with<T>(self, f: impl FnOnce(*const mpv_node) -> T) -> T {
        let args = MpvNodeList::new(self);
        args.with(f)
    }
}
