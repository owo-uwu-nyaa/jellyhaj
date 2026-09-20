pub mod events;
mod helper;
pub use helper::MpvExt;
pub mod nodes;
#[cfg(feature = "stream")]
pub mod stream;
#[cfg(feature = "valuable")]
mod valuable;

use std::{
    cell::UnsafeCell,
    ffi::{CStr, c_int, c_uint, c_ulong, c_void},
    fmt::{Debug, Display},
    mem::{self, ManuallyDrop},
    ptr::{self, NonNull, null},
    task::{Context, Poll, Waker},
    time::Instant,
};

use arcshift::ArcShift;
use mpv_sys::{
    HEADER_MPV_CLIENT_API_VERSION, mpv_client_name, mpv_command_node, mpv_command_node_async,
    mpv_create, mpv_create_client, mpv_create_weak_client, mpv_del_property, mpv_error, mpv_format,
    mpv_get_property, mpv_get_property_async, mpv_handle, mpv_hook_add, mpv_load_config_file,
    mpv_node, mpv_observe_property, mpv_request_log_messages, mpv_set_property,
    mpv_set_property_async, mpv_set_wakeup_callback, mpv_terminate_destroy, mpv_unobserve_property,
    mpv_wait_event,
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

fn check_mpv_version() -> Result<()> {
    #[allow(clippy::cast_possible_truncation)]
    const fn major_minor(v: c_ulong) -> (u16, u16) {
        let minor_mask: c_ulong = 0xffff;
        let major_mask: c_ulong = 0xffff_0000;
        let minor = v & minor_mask;
        let major = v & major_mask;
        let major = major >> 16;
        (major as u16, minor as u16)
    }
    let (h_major, h_minor) = major_minor(HEADER_MPV_CLIENT_API_VERSION);
    let (a_major, a_minor) = major_minor(unsafe { mpv_sys::mpv_client_api_version() });
    #[cfg(feature = "tracing")]
    tracing::info!(
        "Detected mpv client api version {a_major}.{a_minor}. Compiled against {h_major}.{h_minor}"
    );
    'err: {
        if h_major != a_major {
            #[cfg(feature = "tracing")]
            tracing::error!(
                header = h_major,
                library = a_major,
                "Mpv cliant api major version mismatch detected"
            );
            break 'err;
        }
        if h_minor > a_minor {
            #[cfg(feature = "tracing")]
            tracing::error!(
                header = h_minor,
                library = a_minor,
                "Mpv client api minor version mismatch detected"
            );
            break 'err;
        }
        return Ok(());
    };
    Err(Error::new(mpv_error::MPV_ERROR_UNSUPPORTED))
}

unsafe extern "C" fn wakeup_callback(d: *mut c_void) {
    //TODO investigate multi threading
    let arc_shift = unsafe { d.cast::<ArcShift<Option<Waker>>>().as_mut_unchecked() };
    let waker = arc_shift;
    if let Some(waker) = waker.get() {
        waker.wake_by_ref();
    }
}

mod mpv_state {
    use crate::{Initialized, Initializing};

    pub trait StateSealed {}
    impl StateSealed for Initialized {}
    impl StateSealed for Initializing {}
}

pub trait State: mpv_state::StateSealed {}
pub struct Initializing;
impl State for Initializing {}
pub struct Initialized {
    pub waker_set: ArcShift<Option<Waker>>,
    pub waker_callback: Box<UnsafeCell<ArcShift<Option<Waker>>>>,
}
impl State for Initialized {}
unsafe impl Sync for Initialized {}

pub struct Mpv<S: State = Initialized> {
    handle: NonNull<mpv_handle>,
    state: S,
}

impl<S: State> Mpv<S> {
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
    pub fn command(&self, args: impl MpvCommandArg) -> Result<MpvOwnedNode> {
        let mut res: mpv_node = unsafe { mem::zeroed() };
        get_err(args.with(|p| unsafe {
            mpv_command_node(self.handle.as_ptr(), p.cast_mut(), &raw mut res)
        }))?;
        Ok(unsafe { MpvOwnedNode::new(res) })
    }
    pub fn terminate(self) {
        let mut this = ManuallyDrop::new(self);
        unsafe {
            mpv_terminate_destroy(this.handle.as_ptr());
            ptr::drop_in_place(&raw mut this.state);
        }
    }
}

impl Mpv<Initializing> {
    pub fn load_config_file(&self, filename: &CStr) -> Result<()> {
        get_err(unsafe { mpv_load_config_file(self.handle.as_ptr(), filename.as_ptr()) })
    }

    pub fn initialize(self) -> Result<Mpv> {
        get_err(unsafe { mpv_sys::mpv_initialize(self.handle.as_ptr()) })?;
        let waker_set = ArcShift::new(None);
        let waker_callback = Box::new(UnsafeCell::new(waker_set.clone()));
        unsafe {
            mpv_set_wakeup_callback(
                self.handle.as_ptr(),
                Some(wakeup_callback),
                waker_callback.get().cast(),
            );
        }
        let handle = self.handle;
        mem::forget(self);
        Ok(Mpv {
            handle,
            state: Initialized {
                waker_set,
                waker_callback,
            },
        })
    }
}

impl Mpv {
    pub fn new() -> Result<Mpv<Initializing>> {
        check_mpv_version()?;
        let handle = unsafe { NonNull::new(mpv_create()) }
            .ok_or(Error::new(mpv_error::MPV_ERROR_GENERIC))?;
        Ok(Mpv {
            handle,
            state: Initializing,
        })
    }

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
    pub fn get_property_async<F: FromFormat>(
        &self,
        name: &CStr,
        reply_userdata: u64,
    ) -> Result<()> {
        get_err(unsafe {
            mpv_get_property_async(
                self.handle.as_ptr(),
                reply_userdata,
                name.as_ptr(),
                F::FORMAT,
            )
        })
    }
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
    pub fn del_property(&self, name: &CStr) -> Result<()> {
        get_err(unsafe { mpv_del_property(self.handle.as_ptr(), name.as_ptr()) })
    }
    fn observe_property_inner(
        &self,
        reply_userdata: u64,
        name: &CStr,
        format: mpv_format,
    ) -> Result<()> {
        get_err(unsafe {
            mpv_observe_property(self.handle.as_ptr(), reply_userdata, name.as_ptr(), format)
        })
    }
    #[inline]
    pub fn observe_property(
        &self,
        reply_userdata: u64,
        name: &CStr,
        format: MpvFormat,
    ) -> Result<()> {
        self.observe_property_inner(reply_userdata, name, format.ffi())
    }

    pub fn unobserve_property(&self, reply_userdata: u64) -> Result<c_uint> {
        let res = unsafe { mpv_unobserve_property(self.handle.as_ptr(), reply_userdata) };
        c_uint::try_from(res).map_err(|_| Error::new(mpv_error(res)))
    }

    pub fn clone_client(&self, name: Option<&CStr>) -> Result<Self> {
        let name = name.map_or_else(null, CStr::as_ptr);
        let handle = NonNull::new(unsafe { mpv_create_client(self.handle.as_ptr(), name) })
            .ok_or(Error::new(mpv_error::MPV_ERROR_GENERIC))?;
        let waker_set = ArcShift::new(None);
        let waker_callback = Box::new(UnsafeCell::new(waker_set.clone()));
        unsafe {
            mpv_set_wakeup_callback(
                handle.as_ptr(),
                Some(wakeup_callback),
                waker_callback.get().cast(),
            );
        }
        Ok(Self {
            handle,
            state: Initialized {
                waker_set,
                waker_callback,
            },
        })
    }
    pub fn clone_client_weak(&self, name: Option<&CStr>) -> Result<Self> {
        let name = name.map_or_else(null, CStr::as_ptr);
        let handle = NonNull::new(unsafe { mpv_create_weak_client(self.handle.as_ptr(), name) })
            .ok_or(Error::new(mpv_error::MPV_ERROR_GENERIC))?;
        let waker_set = ArcShift::new(None);
        let waker_callback = Box::new(UnsafeCell::new(waker_set.clone()));
        unsafe {
            mpv_set_wakeup_callback(
                handle.as_ptr(),
                Some(wakeup_callback),
                waker_callback.get().cast(),
            );
        }
        Ok(Self {
            handle,
            state: Initialized {
                waker_set,
                waker_callback,
            },
        })
    }

    pub fn command_async(&self, args: &MpvNodeList, reply_userdata: u64) -> Result<()> {
        get_err(args.with(|p| unsafe {
            mpv_command_node_async(self.handle.as_ptr(), reply_userdata, p.cast_mut())
        }))
    }

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
    #[must_use]
    pub fn client_name(&self) -> &CStr {
        unsafe { CStr::from_ptr(mpv_client_name(self.handle.as_ptr())) }
    }
    pub fn request_log_messages(&self, min_level: &CStr) -> Result<()> {
        get_err(unsafe { mpv_request_log_messages(self.handle.as_ptr(), min_level.as_ptr()) })
    }

    unsafe fn unsafe_wait_event<'s>(&mut self, timeout: f64) -> Result<Option<MpvEvent<'s>>> {
        let event = unsafe { mpv_wait_event(self.handle.as_ptr(), timeout).as_ref_unchecked() };
        unsafe { MpvEvent::new(event, self.handle) }
    }
    pub fn wait_event(&mut self) -> Result<MpvEvent<'_>> {
        loop {
            if let Some(res) = unsafe { self.unsafe_wait_event(-1.0) }? {
                break Ok(res);
            }
        }
    }
    pub fn wait_event_timeout_until(&mut self, until: Instant) -> Result<Option<MpvEvent<'_>>> {
        loop {
            let Some(time) = until.checked_duration_since(Instant::now()) else {
                break Ok(None);
            };
            if let Some(res) = unsafe { self.unsafe_wait_event(time.as_secs_f64()) }? {
                break Ok(Some(res));
            }
        }
    }
    unsafe fn unsafe_poll_wait_event<'s>(
        &mut self,
        cx: &mut Context<'_>,
    ) -> Poll<Result<MpvEvent<'s>>> {
        match unsafe { self.unsafe_wait_event(0.0) } {
            Err(e) => return Poll::Ready(Err(e)),
            Ok(None) => {}
            Ok(Some(v)) => return Poll::Ready(Ok(v)),
        }
        self.state.waker_set.rcu_maybe(|prev| {
            let waker = cx.waker();
            if let Some(prev) = prev
                && prev.will_wake(waker)
            {
                None
            } else {
                Some(Some(waker.clone()))
            }
        });
        match unsafe { self.unsafe_wait_event(0.0) } {
            Err(e) => Poll::Ready(Err(e)),
            Ok(None) => Poll::Pending,
            Ok(Some(v)) => Poll::Ready(Ok(v)),
        }
    }

    pub fn poll_wait_event<'s>(&'s mut self, cx: &mut Context<'_>) -> Poll<Result<MpvEvent<'s>>> {
        unsafe { self.unsafe_poll_wait_event(cx) }
    }

    pub const fn wait_event_async(&mut self) -> WaitEvent<'_> {
        WaitEvent { client: self }
    }

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

    pub fn stop_waking(&mut self) {
        self.state
            .waker_set
            .rcu_maybe(|v| if v.is_some() { Some(None) } else { None });
    }
}

impl<S: State> Drop for Mpv<S> {
    fn drop(&mut self) {
        unsafe { mpv_sys::mpv_destroy(self.handle.as_ptr()) };
    }
}

pub struct WaitEvent<'s> {
    client: &'s mut Mpv,
}

impl<'s> Future for WaitEvent<'s> {
    type Output = Result<MpvEvent<'s>>;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        unsafe { self.get_mut().client.unsafe_poll_wait_event(cx) }
    }
}

pub struct Error {
    pub error: mpv_error,
    pub reply_userdata: Option<u64>,
}

impl Error {
    #[must_use]
    pub const fn new(error: mpv_error) -> Self {
        Self {
            error,
            reply_userdata: None,
        }
    }
    #[must_use]
    pub const fn new_reply(error: mpv_error, userdata: u64) -> Self {
        Self {
            error,
            reply_userdata: Some(userdata),
        }
    }
}
#[must_use]
pub fn get_error_string(error: mpv_error) -> &'static CStr {
    unsafe { CStr::from_ptr(mpv_sys::mpv_error_string(error.0)) }
}
impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(userdata) = self.reply_userdata {
            f.write_str("[reply_userdata=")?;
            Display::fmt(&userdata, f)?;
            f.write_str("] ")?;
        }
        f.write_str(
            get_error_string(self.error)
                .to_str()
                .expect("mpv error string not utf8"),
        )
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
        Err(Error::new(v))
    } else {
        Ok(())
    }
}
fn get_err_reply(v: c_int, reply: u64) -> Result<()> {
    let v = mpv_error(v);
    if v < mpv_error::MPV_ERROR_SUCCESS {
        Err(Error::new_reply(v, reply))
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
