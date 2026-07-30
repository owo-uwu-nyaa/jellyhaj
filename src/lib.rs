pub mod widget_creators;

use std::{
    path::PathBuf,
    ptr::NonNull,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicPtr, Ordering::SeqCst},
    },
};

use color_eyre::{
    Result, Section, SectionExt,
    eyre::{Context, eyre},
};
use config::{Config, init_config};
use jellyhaj_core::{
    context::TuiContext,
    state::NextScreen,
    widgets::state::{StateStack, render_loop},
};
use jellyhaj_event_listener::JellyfinEventInterests;
use jellyhaj_image::{Stats, cache::ImageCache};
use keybinds::KeybindEvents;
use player_core::OwnedPlayerHandle;
use player_jellyfin::player_jellyfin;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use spawn::Spawner;
use sqlx::SqliteConnection;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error_span, info, instrument};

use crate::widget_creators::make_screen_login;

#[instrument(skip_all, level = "debug")]
async fn run_state(term: &mut DefaultTerminal, events: &mut KeybindEvents, cx: TuiContext) {
    let widget_creator = {
        let cx = cx.clone();
        Arc::new(move |next| widget_creators::make_screen(next, cx.clone()))
    };
    info!("reached main application loop");
    render_loop(
        NextScreen::LoadHomeScreen,
        widget_creator,
        &cx.state,
        term,
        events,
    )
    .await;
    info!("main application loop exit")
}

async fn run_app_inner(
    mut term: DefaultTerminal,
    mut events: KeybindEvents,
    spawner: Spawner,
    config: Config,
    cache: Arc<tokio::sync::Mutex<SqliteConnection>>,
    image_picker: Picker,
    stop: CancellationToken,
) -> Result<()> {
    let config = Arc::new(config);
    let stats: Stats = Arc::default();
    debug!("logging in to jellyfin");
    if let Some(jellyfin) = jellyhaj_login_view::login(
        config.clone(),
        cache.clone(),
        spawner.clone(),
        stats.clone(),
        make_screen_login,
        &mut term,
        &mut events,
    )
    .await
    {
        let jellyfin_events = JellyfinEventInterests::new(
            &spawner,
            &jellyfin,
            cache.clone(),
            config.dev_store_jellyfin_events,
        )?;
        let mpv_handle = OwnedPlayerHandle::new(
            jellyfin.clone(),
            &config.hwdec,
            config.mpv_profile,
            &config.mpv_log_level,
            config.mpv_config_file.as_deref(),
            true,
            &spawner,
        )?;
        spawner.spawn(
            player_jellyfin(mpv_handle.clone(), jellyfin.clone(), spawner.clone()),
            error_span!("player_jellyfin"),
            "player_jellyfin",
        );
        #[cfg(feature = "mpris")]
        spawner.spawn_res(
            player_mpris::run_mpris_service(mpv_handle.clone(), jellyfin.clone(), stop),
            error_span!("player_mpris"),
            "player_mpris",
        );
        #[cfg(not(feature = "mpris"))]
        let _ = stop;
        run_state(
            &mut term,
            &mut events,
            TuiContext {
                jellyfin,
                jellyfin_events,
                config,
                cache,
                image_cache: ImageCache::new(),
                mpv_handle: mpv_handle.clone(),
                image_picker: Arc::new(image_picker),
                stats,
                spawn: spawner,
                state: Arc::new(StateStack::new()),
            },
        )
        .await;
    }
    Ok(())
}

pub struct AtomicStr {
    inner: AtomicPtr<String>,
}
impl Default for AtomicStr {
    fn default() -> Self {
        Self {
            inner: AtomicPtr::new(std::ptr::null_mut()),
        }
    }
}
#[allow(clippy::box_collection)]
unsafe fn str_box_from_raw(val: *mut String) -> Option<Box<String>> {
    NonNull::new(val).map(|v| unsafe { Box::from_raw(v.as_ptr()) })
}
impl AtomicStr {
    pub fn set(&self, val: String) {
        let new_val = Box::into_raw(Box::new(val));
        let prev_val = self.inner.swap(new_val, SeqCst);
        let _ = unsafe { str_box_from_raw(prev_val) };
    }
    pub fn take(&self) -> Option<String> {
        unsafe { str_box_from_raw(self.inner.swap(std::ptr::null_mut(), SeqCst)) }.map(|v| *v)
    }
}

#[instrument(skip_all, level = "debug")]
#[tokio::main(flavor = "current_thread")]
pub async fn run_app(
    term: DefaultTerminal,
    cancel: CancellationToken,
    stop: CancellationToken,
    paniced: Arc<AtomicStr>,
    config_file: Option<PathBuf>,
    use_builtin_config: bool,
) -> Result<()> {
    let signal_cancel = cancel.clone();
    let interrupted = Arc::new(AtomicBool::new(false));
    let interrupted_flag = interrupted.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("interrupt received");
        interrupted_flag.store(true, SeqCst);
        //produce coredump
        #[cfg(unix)]
        {
            if let Ok(res) = unsafe { nix::unistd::fork() } {
                if matches!(res, nix::unistd::ForkResult::Child) {
                    let _ = nix::sys::signal::raise(nix::sys::signal::SIGTRAP);
                    std::process::abort()
                }
                info!("produced coredump");
            }
        }
        signal_cancel.cancel();
    });
    let cache = config::cache().await?;
    let config = init_config(config_file, use_builtin_config)?;
    let image_picker =
        Picker::from_query_stdio().context("getting information for image display")?;
    let events = KeybindEvents::new()?;

    let res = spawn::run_with_spawner(
        |spawner| {
            run_app_inner(
                term,
                events,
                spawner,
                config,
                cache.clone(),
                image_picker,
                stop.clone(),
            )
        },
        cancel,
        error_span!("jellyhaj"),
        "jellyhaj_main",
    )
    .await;
    if stop.is_cancelled() {
        res.unwrap_or(Ok(()))
    } else {
        res.ok_or_else(move || {
            if let Some(panic_message) = paniced.take() {
                eyre!("Application paniced").section(panic_message.header("Panic message"))
            } else if interrupted.load(SeqCst) {
                eyre!("Application interrupted by signal")
            } else {
                eyre!("Application cancelled for unknown reason")
            }
        })?
    }
}
