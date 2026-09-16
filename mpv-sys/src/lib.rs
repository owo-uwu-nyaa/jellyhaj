#![allow(
    non_camel_case_types,
    non_upper_case_globals,
    clippy::pub_underscore_fields,
    clippy::unreadable_literal
)]
#![cfg_attr(feature = "bindgen", allow(clippy::doc_markdown, clippy::use_self))]

/*!
 * Mechanisms provided by this API
 * -------------------------------
 *
 * This API provides general control over mpv playback. It does not give you
 * direct access to individual components of the player, only the whole thing.
 * It's somewhat equivalent to `MPlayer`'s slave mode. You can send commands,
 * retrieve or set playback status or settings with properties, and receive
 * events.
 *
 * The API can be used in two ways:
 * 1) Internally in mpv, to provide additional features to the command line
 *    player. Lua scripting uses this. (Currently there is no plugin API to
 *    get a client API handle in external user code. It has to be a fixed
 *    part of the player at compilation time.)
 * 2) Using mpv as a library with `mpv_create()`. This basically allows embedding
 *    mpv in other applications.
 *
 * Documentation
 * -------------
 *
 * The libmpv C API is documented directly in this header. Note that most
 * actual interaction with this player is done through
 * options/commands/properties, which can be accessed through this API.
 * Essentially everything is done with them, including loading a file,
 * retrieving playback progress, and so on.
 *
 * These are documented elsewhere:
 * - <http://mpv.io/manual/master/#options>
 * - <http://mpv.io/manual/master/#list-of-input-commands>
 * - <http://mpv.io/manual/master/#properties>
 *
 * You can also look at the examples here:
 *      * <https://github.com/mpv-player/mpv-examples/tree/master/libmpv>
 *
 * Event loop
 * ----------
 *
 * In general, the API user should run an event loop in order to receive events.
 * This event loop should call `mpv_wait_event()`, which will return once a new
 * mpv client API is available. It is also possible to integrate client API
 * usage in other event loops (e.g. GUI toolkits) with the
 * `mpv_set_wakeup_callback()` function, and then polling for events by calling
 * `mpv_wait_event()` with a 0 timeout.
 *
 * Note that the event loop is detached from the actual player. Not calling
 * `mpv_wait_event()` will not stop playback. It will eventually congest the
 * event queue of your API handle, though.
 *
 * Synchronous vs. asynchronous calls
 * ----------------------------------
 *
 * The API allows both synchronous and asynchronous calls. Synchronous calls
 * have to wait until the playback core is ready, which currently can take
 * an unbounded time (e.g. if network is slow or unresponsive). Asynchronous
 * calls just queue operations as requests, and return the result of the
 * operation as events.
 *
 * Asynchronous calls
 * ------------------
 *
 * The client API includes asynchronous functions. These allow you to send
 * requests instantly, and get replies as events at a later point. The
 * requests are made with functions carrying the _async suffix, and replies
 * are returned by `mpv_wait_event()` (interleaved with the normal event stream).
 *
 * A 64 bit userdata value is used to allow the user to associate requests
 * with replies. The value is passed as `reply_userdata` parameter to the request
 * function. The reply to the request will have the reply
 * mpv_event->reply_userdata field set to the same value as the
 * `reply_userdata` parameter of the corresponding request.
 *
 * This userdata value is arbitrary and is never interpreted by the API. Note
 * that the userdata value 0 is also allowed, but then the client must be
 * careful not accidentally interpret the mpv_event->reply_userdata if an
 * event is not a reply. (For non-replies, this field is set to 0.)
 *
 * Asynchronous calls may be reordered in arbitrarily with other synchronous
 * and asynchronous calls. If you want a guaranteed order, you need to wait
 * until asynchronous calls report completion before doing the next call.
 *
 * See also the section "Asynchronous command details" in the manpage.
 *
 * Multithreading
 * --------------
 *
 * The client API is generally fully thread-safe, unless otherwise noted.
 * Currently, there is no real advantage in using more than 1 thread to access
 * the client API, since everything is serialized through a single lock in the
 * playback core.
 *
 * Basic environment requirements
 * ------------------------------
 *
 * This documents basic requirements on the C environment. This is especially
 * important if mpv is used as library with `mpv_create()`.
 *
 * - The `LC_NUMERIC` locale category must be set to "C". If your program calls
 *   `setlocale()`, be sure not to use `LC_ALL`, or if you do, reset `LC_NUMERIC`
 *   to its sane default: `setlocale(LC_NUMERIC`, "C").
 * - If a X11 based VO is used, mpv will set the xlib error handler. This error
 *   handler is process-wide, and there's no proper way to share it with other
 *   xlib users within the same process. This might confuse GUI toolkits.
 * - mpv uses some other libraries that are not library-safe, such as Fribidi
 *   (used through libass), ALSA, `FFmpeg`, and possibly more.
 * - The FPU precision must be set at least to double precision.
 * - On Windows, mpv will call timeBeginPeriod(1).
 * - On memory exhaustion, mpv will kill the process.
 * - In certain cases, mpv may start sub processes (such as with the ytdl
 *   wrapper script).
 * - Using UNIX IPC (off by default) will override the SIGPIPE signal handler,
 *   and set it to `SIG_IGN`. Some invocations of the "subprocess" command will
 *   also do that.
 * - mpv may start sub processes, so overriding SIGCHLD, or waiting on all PIDs
 *   (such as calling `wait()`) by the parent process or any other library within
 *   the process must be avoided. libmpv itself only waits for its own PIDs.
 * - If anything in the process registers signal handlers, they must set the
 *   `SA_RESTART` flag. Otherwise you WILL get random failures on signals.
 *
 * Encoding of filenames
 * ---------------------
 *
 * mpv uses UTF-8 everywhere.
 *
 * On some platforms (like Linux), filenames actually do not have to be UTF-8;
 * for this reason libmpv supports non-UTF-8 strings. libmpv uses what the
 * kernel uses and does not recode filenames. At least on Linux, passing a
 * string to libmpv is like passing a string to the `fopen()` function.
 *
 * On Windows, filenames are always UTF-8, libmpv converts between UTF-8 and
 * UTF-16 when using win32 API functions. libmpv never uses or accepts
 * filenames in the local 8 bit encoding. It does not use `fopen()` either;
 * it uses _`wfopen()`.
 *
 * On macOS, filenames and other strings taken/returned by libmpv can have
 * inconsistent unicode normalization. This can sometimes lead to problems.
 * You have to hope for the best.
 *
 * Also see the remarks for `MPV_FORMAT_STRING`.
 *
 * Embedding the video window
 * --------------------------
 *
 * Using the render API (in render.h) is recommended. This API requires
 * you to create and maintain an OpenGL context, to which you can render
 * video using a specific API call. This API does not include keyboard or mouse
 * input directly.
 *
 * There is an older way to embed the native mpv window into your own. You have
 * to get the raw window handle, and set it as "wid" option. This works on X11,
 * win32, and macOS only. It's much easier to use than the render API, but
 * also has various problems.
 *
 * Also see client API examples and the mpv manpage. There is an extensive
 * discussion here:
 * <https://github.com/mpv-player/mpv-examples/tree/master/libmpv#methods-of-embedding-the-video-window>
 *
 * Compatibility
 * -------------
 *
 * mpv development doesn't stand still, and changes to mpv internals as well as
 * to its interface can cause compatibility issues to client API users.
 *
 * The API is versioned (see `MPV_CLIENT_API_VERSION`), and changes to it are
 * documented in DOCS/client-api-changes.rst. The C API itself will probably
 * remain compatible for a long time, but the functionality exposed by it
 * could change more rapidly. For example, it's possible that options are
 * renamed, or change the set of allowed values.
 *
 * Defensive programming should be used to potentially deal with the fact that
 * options, commands, and properties could disappear, change their value range,
 * or change the underlying datatypes. It might be a good idea to prefer
 * `MPV_FORMAT_STRING` over other types to decouple your code from potential
 * mpv changes.
 *
 * Also see: DOCS/compatibility.rst
 *
 * Future changes
 * --------------
 *
 * This are the planned changes that will most likely be done on the next major
 * bump of the library:
 *
 *  - remove all symbols that are marked as deprecated
 *  - reassign enum numerical values to remove gaps
 *  - disabling all events by default
 */

#[cfg(not(feature = "bindgen"))]
include!("client.rs");
#[cfg(feature = "bindgen")]
include!(concat!(env!("OUT_DIR"), "/client.rs"));

pub mod render {
    /*!
     * Overview
     * --------
     *
     * This API can be used to make mpv render using supported graphic APIs (such
     * as OpenGL). It can be used to handle video display.
     *
     * The renderer needs to be created with `mpv_render_context_create()` before
     * you start playback (or otherwise cause a VO to be created). Then (with most
     * backends) `mpv_render_context_render()` can be used to explicitly render the
     * current video frame. Use `mpv_render_context_set_update_callback()` to get
     * notified when there is a new frame to draw.
     *
     * Preferably rendering should be done in a separate thread. If you call
     * normal libmpv API functions on the renderer thread, deadlocks can result
     * (these are made non-fatal with timeouts, but user experience will obviously
     * suffer). See "Threading" section below.
     *
     * You can output and embed video without this API by setting the mpv "wid"
     * option to a native window handle (see "Embedding the video window" section
     * in the client.h header). In general, using the render API is recommended,
     * because window embedding can cause various issues, especially with GUI
     * toolkits and certain platforms.
     *
     * Supported backends
     * ------------------
     *
     * OpenGL: via `MPV_RENDER_API_TYPE_OPENGL`, see `render_gl.h` header.
     * Software: via `MPV_RENDER_API_TYPE_SW`, see section "Software renderer"
     *
     * Threading
     * ---------
     *
     * You are recommended to do rendering on a separate thread than normal libmpv
     * use.
     *
     * The `mpv_render`_* functions can be called from any thread, under the
     * following conditions:
     *  - only one of the `mpv_render`_* functions can be called at the same time
     *    (unless they belong to different mpv cores created by `mpv_create()`)
     *  - never can be called from within the callbacks set with
     *    `mpv_set_wakeup_callback()` or `mpv_render_context_set_update_callback()`
     *  - if the OpenGL backend is used, for all functions the OpenGL context
     *    must be "current" in the calling thread, and it must be the same OpenGL
     *    context as the `mpv_render_context` was created with. Otherwise, undefined
     *    behavior will occur.
     *  - the thread does not call libmpv API functions other than the `mpv_render`_*
     *    functions, except APIs which are declared as safe (see below). Likewise,
     *    there must be no lock or wait dependency from the render thread to a
     *    thread using other libmpv functions. Basically, the situation that your
     *    render thread waits for a "not safe" libmpv API function to return must
     *    not happen. If you ignore this requirement, deadlocks can happen, which
     *    are made non-fatal with timeouts; then playback quality will be degraded,
     *    and the message `mpv_render_context_render()` not being called or stuck.
     *    is logged. If you set `MPV_RENDER_PARAM_ADVANCED_CONTROL`, you promise that
     *    this won't happen, and must absolutely guarantee it, or a real deadlock
     *    will freeze the mpv core thread forever.
     *
     * libmpv functions which are safe to call from a render thread are:
     *  - functions marked with "Safe to be called from mpv render API threads."
     *  - client.h functions which don't have an explicit or implicit `mpv_handle`
     *    parameter
     *  - `mpv_render`_* functions; but only for the same `mpv_render_context` pointer.
     *    If the pointer is different, `mpv_render_context_free()` is not safe. (The
     *    reason is that if `MPV_RENDER_PARAM_ADVANCED_CONTROL` is set, it may have
     *    to process still queued requests from the core, which it can do only for
     *    the current context, while requests for other contexts would deadlock.
     *    Also, it may have to wait and block for the core to terminate the video
     *    chain to make sure no resources are used after context destruction.)
     *  - if the `mpv_handle` parameter refers to a different mpv core than the one
     *    you're rendering for (very obscure, but allowed)
     *
     * Note about old libmpv version:
     *
     *      Before API version 1.105 (basically in mpv 0.29.x), simply enabling
     *      MPV_RENDER_PARAM_ADVANCED_CONTROL could cause deadlock issues. This can
     *      be worked around by setting the "vd-lavc-dr" option to "no".
     *      In addition, you were required to call all mpv_render*() API functions
     *      from the same thread on which mpv_render_context_create() was originally
     *      run (for the same the mpv_render_context). Not honoring it led to UB
     *      (deadlocks, use of invalid mp_thread handles), even if you moved your GL
     *      context to a different thread correctly.
     *      These problems were addressed in API version 1.105 (mpv 0.30.0).
     *
     * Context and handle lifecycle
     * ----------------------------
     *
     * Video initialization will fail if the render context was not initialized yet
     * (with `mpv_render_context_create()`), or it will revert to a VO that creates
     * its own window.
     *
     * Currently, there can be only 1 `mpv_render_context` at a time per mpv core.
     *
     * Calling `mpv_render_context_free()` while a VO is using the render context is
     * active will disable video.
     *
     * You must free the context with `mpv_render_context_free()` before the mpv core
     * is destroyed. If this doesn't happen, undefined behavior will result.
     *
     * Software renderer
     * -----------------
     *
     * `MPV_RENDER_API_TYPE_SW` provides an extremely simple (but slow) renderer to
     * memory surfaces. You probably don't want to use this. Use other render API
     * types, or other methods of video embedding.
     *
     * Use `mpv_render_context_create()` with `MPV_RENDER_PARAM_API_TYPE` set to
     * `MPV_RENDER_API_TYPE_SW`.
     *
     * Call `mpv_render_context_render()` with various `MPV_RENDER_PARAM_SW`_* fields
     * to render the video frame to an in-memory surface. The following fields are
     * required: `MPV_RENDER_PARAM_SW_SIZE`, `MPV_RENDER_PARAM_SW_FORMAT`,
     * `MPV_RENDER_PARAM_SW_STRIDE`, `MPV_RENDER_PARAM_SW_POINTER`.
     *
     * This method of rendering is very slow, because everything, including color
     * conversion, scaling, and OSD rendering, is done on the CPU, single-threaded.
     * In particular, large video or display sizes, as well as presence of OSD or
     * subtitles can make it too slow for realtime. As with other software rendering
     * VOs, setting "sw-fast" may help. Enabling or disabling zimg may help,
     * depending on the platform.
     *
     * In addition, certain multimedia job creation measures like HDR may not work
     * properly, and will have to be manually handled by for example inserting
     * filters.
     *
     * This API is not really suitable to extract individual frames from video etc.
     * (basically non-playback uses) - there are better libraries for this. It can
     * be used this way, but it may be clunky and tricky.
     *
     * Further notes:
     * - `MPV_RENDER_PARAM_FLIP_Y` is currently ignored (unsupported)
     * - `MPV_RENDER_PARAM_DEPTH` is ignored (meaningless)
     *
     *
     * OpenGL backend
     * --------------
     *
     * This header contains definitions for using OpenGL with the render.h API.
     *
     * OpenGL interop
     * --------------
     *
     * The OpenGL backend has some special rules, because OpenGL itself uses
     * implicit per-thread contexts, which causes additional API problems.
     *
     * This assumes the OpenGL context lives on a certain thread controlled by the
     * API user. All `mpv_render`_* APIs have to be assumed to implicitly use the
     * OpenGL context if you pass a `mpv_render_context` using the OpenGL backend,
     * unless specified otherwise.
     *
     * The OpenGL context is indirectly accessed through the OpenGL function
     * pointers returned by the `get_proc_address` callback in `mpv_opengl_init_params`.
     * Generally, mpv will not load the system OpenGL library when using this API.
     *
     * OpenGL state
     * ------------
     *
     * OpenGL has a large amount of implicit state. All the mpv functions mentioned
     * above expect that the OpenGL state is reasonably set to OpenGL standard
     * defaults. Likewise, mpv will attempt to leave the OpenGL context with
     * standard defaults. The following state is excluded from this:
     *
     *      - the glViewport state
     *      - the glScissor state (but GL_SCISSOR_TEST is in its default value)
     *      - glBlendFuncSeparate() state (but GL_BLEND is in its default value)
     *      - glClearColor() state
     *      - mpv may overwrite the callback set with glDebugMessageCallback()
     *      - mpv always disables GL_DITHER at init
     *
     * Messing with the state could be avoided by creating shared OpenGL contexts,
     * but this is avoided for the sake of compatibility and interoperability.
     *
     * On OpenGL 2.1, mpv will strictly call functions like `glGenTextures()` to
     * create OpenGL objects. You will have to do the same. This ensures that
     * objects created by mpv and the API users don't clash. Also, legacy state
     * must be either in its defaults, or not interfere with core state.
     *
     * API use
     * -------
     *
     * The `mpv_render`_* API is used. That API supports multiple backends, and this
     * section documents specifics for the OpenGL backend.
     *
     * Use `mpv_render_context_create()` with `MPV_RENDER_PARAM_API_TYPE` set to
     * `MPV_RENDER_API_TYPE_OPENGL`, and `MPV_RENDER_PARAM_OPENGL_INIT_PARAMS` provided.
     *
     * Call `mpv_render_context_render()` with `MPV_RENDER_PARAM_OPENGL_FBO` to render
     * the video frame to an FBO.
     *
     * Hardware decoding
     * -----------------
     *
     * Hardware decoding via this API is fully supported, but requires some
     * additional setup. (At least if direct hardware decoding modes are wanted,
     * instead of copying back surface data from GPU to CPU RAM.)
     *
     * There may be certain requirements on the OpenGL implementation:
     *
     * - Windows: ANGLE is required (although in theory GL/DX interop could be used)
     * - Intel/Linux: EGL is required, and also the native display resource needs to be provided (e.g. `MPV_RENDER_PARAM_X11_DISPLAY` for X11 and `MPV_RENDER_PARAM_WL_DISPLAY` for Wayland)
     * - nVidia/Linux: Both GLX and EGL should work (GLX is required if vdpau is used, e.g. due to old drivers.)
     * - macOS: CGL is required (`CGLGetCurrentContext()` returning non-NULL)
     * - iOS: EAGL is required (EAGLContext.currentContext returning non-nil)
     *
     * Once these things are setup, hardware decoding can be enabled/disabled at
     * any time by setting the "hwdec" property.
     */

    #[cfg(not(feature = "bindgen"))]
    include!("render.rs");
    #[cfg(feature = "bindgen")]
    include!(concat!(env!("OUT_DIR"), "/render.rs"));
    use crate::mpv_handle;
}

pub mod stream_cb {
    /*!
     * Warning: this API is not stable yet.
     *
     * Overview
     * --------
     *
     * This API can be used to make mpv read from a stream with a custom
     * implementation. This interface is inspired by funopen on BSD and
     * fopencookie on linux. The stream is backed by user-defined callbacks
     * which can implement customized open, read, seek, size and close behaviors.
     *
     * Usage
     * -----
     *
     * Register your stream callbacks with the `mpv_stream_cb_add_ro()` function. You
     * have to provide a `mpv_stream_cb_open_ro_fn` callback to it (`open_fn` argument).
     *
     * Once registered, you can `loadfile myprotocol://myfile`. Your `open_fn` will be
     * invoked with the URI and you must fill out the provided `mpv_stream_cb_info`
     * struct. This includes your stream callbacks (like `read_fn`), and an opaque
     * cookie, which will be passed as the first argument to all the remaining
     * stream callbacks.
     *
     * Note that your custom callbacks must not invoke libmpv APIs as that would
     * cause a deadlock. (Unless you call a different `mpv_handle` than the one the
     * callback was registered for, and the `mpv_handles` refer to different mpv
     * instances.)
     *
     * Stream lifetime
     * ---------------
     *
     * A stream remains valid until its close callback has been called. It's up to
     * libmpv to call the close callback, and the libmpv user cannot close it
     * directly with the `stream_cb` API.
     *
     * For example, if you consider your custom stream to become suddenly invalid
     * (maybe because the underlying stream died), libmpv will continue using your
     * stream. All you can do is returning errors from each callback, until libmpv
     * gives up and closes it.
     *
     * Protocol registration and lifetime
     * ----------------------------------
     *
     * Protocols remain registered until the mpv instance is terminated. This means
     * in particular that it can outlive the `mpv_handle` that was used to register
     * it, but once `mpv_terminate_destroy()` is called, your registered callbacks
     * will not be called again.
     *
     * Protocol unregistration is finished after the mpv core has been destroyed
     * (e.g. after `mpv_terminate_destroy()` has returned).
     *
     * If you do not call `mpv_terminate_destroy()` yourself (e.g. plugin-style code),
     * you will have to deal with the registration or even streams outliving your
     * code. Here are some possible ways to do this:
     * - call `mpv_terminate_destroy()`, which destroys the core, and will make sure
     *   all streams are closed once this function returns
     * - you refcount all resources your stream "cookies" reference, so that it
     *   doesn't matter if streams live longer than expected
     * - create "cancellation" semantics: after your protocol has been unregistered,
     *   notify all your streams that are still opened, and make them drop all
     *   referenced resources - then return errors from the stream callbacks as
     *   long as the stream is still opened
     *
     */
    #[cfg(not(feature = "bindgen"))]
    include!("stream_cb.rs");
    #[cfg(feature = "bindgen")]
    include!(concat!(env!("OUT_DIR"), "/stream_cb.rs"));
    use crate::mpv_handle;
}
