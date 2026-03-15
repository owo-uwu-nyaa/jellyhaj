use std::{path::PathBuf, sync::Arc};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use config::{Config, init_config};
use jellyhaj_core::{
    context::TuiContext,
    render::{NavigationResult, Suspended},
    state::{Next, NextScreen},
};
use jellyhaj_event_listener::JellyfinEventInterests;
use jellyhaj_image::cache::ImageProtocolCache;
use keybinds::KeybindEvents;
use player_core::OwnedPlayerHandle;
use player_jellyfin::player_jellyfin;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use spawn::Spawner;
use sqlx::SqliteConnection;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error_span, info, instrument};

async fn show_screen(
    screen: Next,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    cx: TuiContext,
) -> NavigationResult {
    match *screen {
        NextScreen::LoadHomeScreen => {
            jellyhaj_home_screen_view::render_fetch_home_screen(term, events, cx).await
        }
        NextScreen::HomeScreen {
            cont,
            next_up,
            libraries,
            library_latest,
        } => {
            jellyhaj_home_screen_view::render_home_screen(
                term,
                events,
                cx,
                cont,
                next_up,
                libraries,
                library_latest,
            )
            .await
        }
        NextScreen::LoadUserView(user_view) => {
            jellyhaj_library_view::render_fetch_user_view(term, events, cx, user_view).await
        }
        NextScreen::UserView { view, items } => {
            jellyhaj_library_view::render_user_view(term, events, cx, view, items).await
        }
        NextScreen::FetchPlay(load_play) => {
            jellyhaj_player_view::render_fetch_play(term, events, cx, load_play).await
        }
        NextScreen::Play { items, index } => {
            jellyhaj_player_view::render_play(term, events, cx, items, index).await
        }
        NextScreen::Error(report) => {
            jellyhaj_error_view::render_error(term, events, cx, report).await
        }
        NextScreen::ItemDetails(media_item) => {
            jellyhaj_item_details_view::render_item_details(term, events, cx, media_item).await
        }
        NextScreen::ItemListDetails(media_item, media_items) => {
            jellyhaj_item_details_view::render_item_list_details(
                term,
                events,
                cx,
                media_item,
                media_items,
            )
            .await
        }
        NextScreen::FetchItemListDetails(media_item) => {
            jellyhaj_item_details_view::render_fetch_item_list(term, events, cx, media_item).await
        }
        NextScreen::FetchItemListDetailsRef(id) => {
            jellyhaj_item_details_view::render_fetch_item_list_ref(term, events, cx, id).await
        }
        NextScreen::FetchItemDetails(item) => {
            jellyhaj_item_details_view::render_fetch_episode(term, events, cx, item).await
        }
        NextScreen::RefreshItem(id) => {
            jellyhaj_refresh_item_view::render_refresh_item_form(term, events, cx, id).await
        }
        NextScreen::Stats => jellyhaj_stats_view::render_stats(term, events, cx).await,
        NextScreen::Logs => jellyhaj_log_view::render_log(term, events, cx).await,
    }
}

#[instrument(skip_all, level = "debug")]
async fn run_state(term: &mut DefaultTerminal, events: &mut KeybindEvents, cx: TuiContext) {
    let mut state: Vec<Suspended> = Vec::new();
    let mut top: Option<Next> = None;
    info!("reached main application loop");
    loop {
        let res = if let Some(top) = top.take() {
            debug!("running top next screen");
            show_screen(top, term, events, cx.clone()).await
        } else if let Some(mut suspended) = state.pop() {
            debug!("resuming suspended widget: {}", suspended.name());
            suspended.resume(term, events).await
        } else {
            debug!("defaulting to displaying home screen");
            jellyhaj_home_screen_view::render_fetch_home_screen(term, events, cx.clone()).await
        };
        match res {
            NavigationResult::Exit => break,
            NavigationResult::Pop => {}
            NavigationResult::Replace(next_screen) => top = Some(next_screen),
            NavigationResult::Push { current, next } => {
                state.push(current);
                top = Some(next);
            }
            NavigationResult::PushWithoutTui {
                current,
                without_tui,
            } => {
                state.push(current);
                if let Err(e) = jellyhaj_core::term::run_without(without_tui).await {
                    top = Some(Box::new(NextScreen::Error(e)))
                }
            }
        }
    }
    info!("main application loop exit")
}

async fn run_app_inner(
    mut term: DefaultTerminal,
    mut events: KeybindEvents,
    spawner: Spawner,
    config: Config,
    cache: Arc<tokio::sync::Mutex<SqliteConnection>>,
    image_picker: Picker,
) -> Result<()> {
    let config = Arc::new(config);
    debug!("logging in to jellyfin");
    if let Some(jellyfin) = jellyhaj_login_view::login(
        clap::crate_name!(),
        clap::crate_version!(),
        &mut term,
        &mut events,
        spawner.clone(),
        config.clone(),
    )
    .await?
    {
        let jellyfin_events = JellyfinEventInterests::new(&spawner, &jellyfin)?;
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
            player_mpris::run_mpris_service(mpv_handle.clone(), jellyfin.clone()),
            error_span!("player_mpris"),
            "player_mpris",
        );
        run_state(
            &mut term,
            &mut events,
            TuiContext {
                jellyfin,
                jellyfin_events,
                config,
                cache,
                image_cache: ImageProtocolCache::new(),
                mpv_handle: mpv_handle.clone(),
                image_picker: Arc::new(image_picker),
                stats: Default::default(),
                spawn: spawner,
            },
        )
        .await
    }
    Ok(())
}

#[instrument(skip_all, level = "debug")]
#[tokio::main(flavor = "current_thread")]
pub async fn run_app(
    term: DefaultTerminal,
    cancel: CancellationToken,
    config_file: Option<PathBuf>,
    use_builtin_config: bool,
) -> Result<()> {
    let signal_cancel = cancel.clone();
    tokio::spawn(async move {
        let _ = tokio::signal::ctrl_c().await;
        info!("interrupt received");
        signal_cancel.cancel();
    });
    let cache = config::cache().await?;
    let config = init_config(config_file, use_builtin_config)?;
    let image_picker =
        Picker::from_query_stdio().context("getting information for image display")?;
    let events = KeybindEvents::new()?;
    spawn::run_with_spawner(
        |spawner| run_app_inner(term, events, spawner, config, cache.clone(), image_picker),
        cancel,
        error_span!("jellyhaj"),
        "jellyhaj_main",
    )
    .await
    .ok_or_eyre("app cancelled")?
}
