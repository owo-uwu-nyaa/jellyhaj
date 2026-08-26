pub mod widget_creators;

use std::{path::PathBuf, rc::Rc, sync::Arc};

use color_eyre::{Result, eyre::Context};
use config::init_config;
use jellyhaj_context::TuiContext;
use jellyhaj_core::{
    state::NextScreen,
    widgets::state::{StateStackHandle, render_loop},
};
use jellyhaj_event_listener::JellyfinEventInterests;
use jellyhaj_image::{Stats, cache::ImageCache};
use jellyhaj_widgets_core::async_task::{UnboundedReceiver, unbounded_channel};
use keybinds::KeybindEvents;
use player_core::OwnedPlayerHandle;
use player_jellyfin::player_jellyfin;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use spawn::Spawner;
use tracing::{debug, error_span, info, instrument};

use crate::widget_creators::make_screen_login;

#[instrument(skip_all, level = "debug")]
async fn run_state(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
    external: &mut UnboundedReceiver<NextScreen>,
) {
    let widget_creator = {
        let cx = cx.clone();
        Rc::new(move |next| widget_creators::make_screen(next, cx.clone()))
    };
    info!("reached main application loop");
    render_loop(
        NextScreen::LoadHomeScreen,
        widget_creator,
        &cx.state,
        term,
        events,
        external,
    )
    .await;
    info!("main application loop exit")
}

#[instrument(skip_all, level = "debug")]
pub async fn run_app(
    mut term: DefaultTerminal,
    spawner: Spawner,
    config_file: Option<PathBuf>,
    use_builtin_config: bool,
) -> Result<()> {
    let cache = config::cache().await?;
    let config = init_config(config_file, use_builtin_config)?;
    let image_picker =
        Picker::from_query_stdio().context("getting information for image display")?;
    let mut events = KeybindEvents::new()?;

    let config = Arc::new(config);
    let stats: Stats = Arc::default();
    debug!("logging in to jellyfin");
    let (widget_sender, mut widget_receiver) = unbounded_channel();
    if let Some(jellyfin) = jellyhaj_login_view::login(
        config.clone(),
        cache.clone(),
        spawner.clone(),
        stats.clone(),
        make_screen_login,
        &mut term,
        &mut events,
        &mut widget_receiver,
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
            widget_sender.clone(),
        )?;
        #[cfg(feature = "mpris")]
        spawner.spawn_res(
            player_mpris::run_mpris_service(
                mpv_handle.clone(),
                jellyfin.clone(),
                widget_sender.clone(),
            ),
            error_span!("player_mpris"),
            "player_mpris",
        );
        spawner.spawn(
            player_jellyfin(mpv_handle.clone(), jellyfin.clone(), spawner.clone()),
            error_span!("player_jellyfin"),
            "player_jellyfin",
        );
        let state_stack = StateStackHandle::new();
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
                image_picker: Rc::new(image_picker),
                stats,
                spawn: spawner,
                state: state_stack.clone(),
            },
            &mut widget_receiver,
        )
        .await;
    }
    Ok(())
}
