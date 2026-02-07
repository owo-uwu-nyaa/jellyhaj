use std::{
    path::PathBuf,
    pin::{Pin, pin},
    sync::Arc,
};

use color_eyre::{
    Result,
    eyre::{Context, eyre},
};
use config::{Config, init_config};
use jellyfin::{JellyfinClient, socket::JellyfinWebSocket};
use jellyhaj_core::{
    context::TuiContext,
    state::{Navigation, NextScreen, State},
};
use jellyhaj_error_view::ResultDisplayExt;
use keybinds::KeybindEvents;
use player_core::OwnedPlayerHandle;
use player_jellyfin::player_jellyfin;
use ratatui::DefaultTerminal;
use ratatui_image::picker::Picker;
use spawn::Spawner;
use sqlx::SqliteConnection;
use tokio_util::sync::CancellationToken;
use tracing::{error_span, info, instrument};

async fn show_screen(screen: NextScreen, cx: Pin<&mut TuiContext>) -> Result<Navigation> {
    match screen {
        NextScreen::LoadHomeScreen => jellyhaj_home_screen_view::render_fetch_home_screen(cx).await,
        NextScreen::HomeScreen(entry_screen) => {
            jellyhaj_home_screen_view::render_home_screen(cx, entry_screen).await
        }
        NextScreen::LoadUserView(user_view) => {
            jellyhaj_library_view::render_fetch_user_view(cx, user_view).await
        }
        NextScreen::UserView { view, items } => {
            jellyhaj_library_view::render_user_view(cx, items, view).await
        }
        NextScreen::Play { items, index } => {
            jellyhaj_player_view::render_play(cx, items, index).await
        }
        NextScreen::Error(report) => {
            let cx = cx.project();
            jellyhaj_error_view::render_error(
                cx.term,
                cx.events,
                &cx.config.keybinds,
                &cx.config.help_prefixes,
                cx.spawn.clone(),
                report,
            )
            .await
        }
        NextScreen::ItemDetails(media_item) => {
            jellyhaj_item_details_view::render_item_details(cx, media_item).await
        }
        NextScreen::ItemListDetails(media_item, entry_list) => {
            jellyhaj_item_details_view::render_item_list_details(cx, media_item, entry_list).await
        }
        NextScreen::FetchItemListDetails(media_item) => {
            jellyhaj_item_details_view::render_fetch_item_list(cx, media_item).await
        }
        NextScreen::FetchItemListDetailsRef(id) => {
            jellyhaj_item_details_view::render_fetch_item_list_ref(cx, id).await
        }
        NextScreen::FetchItemDetails(id) => {
            jellyhaj_item_details_view::render_fetch_episode(cx, id).await
        }
        NextScreen::RefreshItem(item) => {
            jellyhaj_refresh_item_view::render_refresh_item_form(cx, item).await
        }
        NextScreen::SendRefreshItem(item, refresh_item_query) => {
            jellyhaj_refresh_item_view::render_send_refresh_item(cx, item, refresh_item_query).await
        }
        NextScreen::UnsupportedItem => Err(eyre!("Operation is not available for this item")),
        NextScreen::Stats => jellyhaj_stats_view::render_stats(cx).await,
        NextScreen::Logs => jellyhaj_log_view::render_log(cx).await,
        NextScreen::FetchPlay(load_play) => {
            jellyhaj_player_view::render_fetch_play(cx, load_play).await
        }
    }
}

async fn login_jellyfin(
    term: &mut DefaultTerminal,
    spawner: &Spawner,
    events: &mut KeybindEvents,
    config: &Config,
) -> Result<Option<(JellyfinClient, JellyfinWebSocket)>> {
    Ok(
        if let Some(client) = jellyhaj_login_view::login(term, spawner, config, events).await? {
            let socket = client.get_socket()?;
            Some((client, socket))
        } else {
            None
        },
    )
}

#[instrument(skip_all, level = "debug")]
async fn login(
    term: &mut DefaultTerminal,
    spawner: &Spawner,
    events: &mut KeybindEvents,
    config: &Config,
) -> Option<(JellyfinClient, JellyfinWebSocket)> {
    loop {
        match login_jellyfin(term, spawner, events, config).await {
            Ok(v) => break v,
            Err(e) => {
                match jellyhaj_error_view::render_error(
                    term,
                    events,
                    &config.keybinds,
                    &config.help_prefixes,
                    spawner.clone(),
                    e,
                )
                .await
                {
                    Err(_) | Ok(Navigation::Exit) => break None,
                    _ => {}
                }
            }
        }
    }
}

#[instrument(skip_all, level = "debug")]
async fn run_state(mut cx: Pin<&mut TuiContext>) {
    let mut state = State::new();
    info!("reached main application loop");
    while let Some(screen) = state.pop() {
        state.navigate(match show_screen(screen, cx.as_mut()).await {
            Ok(nav) => nav,
            Err(e) => Navigation::Replace(NextScreen::Error(e)),
        });
    }
}

async fn run_app_inner(
    mut term: DefaultTerminal,
    mut events: KeybindEvents,
    spawner: Spawner,
    config: Config,
    cache: Arc<tokio::sync::Mutex<SqliteConnection>>,
    image_picker: Picker,
) {
    if let Some((jellyfin, jellyfin_socket)) =
        login(&mut term, &spawner, &mut events, &config).await
        && let Some(mpv_handle) = OwnedPlayerHandle::new(
            jellyfin.clone(),
            &config.hwdec,
            config.mpv_profile,
            &config.mpv_log_level,
            config.mpv_config_file.as_deref(),
            true,
            &spawner,
        )
        .render_error(
            &mut term,
            &mut events,
            &config.keybinds,
            &config.help_prefixes,
            spawner.clone(),
        )
        .await
    {
        spawner.spawn(
            player_jellyfin(mpv_handle.clone(), jellyfin.clone(), spawner.clone()),
            error_span!("player_jellyfin"),
        );
        #[cfg(feature = "mpris")]
        spawner.spawn_res(
            player_mpris::run_mpris_service(mpv_handle.clone(), jellyfin.clone()),
            error_span!("player_mpris"),
        );
        let cx = pin!(TuiContext {
            jellyfin,
            jellyfin_socket,
            term,
            config,
            events,
            image_picker: Arc::new(image_picker),
            cache,
            image_cache: jellyhaj_image::cache::ImageProtocolCache::new(),
            mpv_handle,
            stats: Default::default(),
            spawn: spawner
        });
        run_state(cx).await
    }
}

#[instrument(skip_all, level = "debug")]
#[tokio::main(flavor = "current_thread")]
pub async fn run_app(
    term: DefaultTerminal,
    cancel: CancellationToken,
    config_file: Option<PathBuf>,
    use_builtin_config: bool,
) -> Result<()> {
    let cache = config::cache().await?;
    let config = init_config(config_file, use_builtin_config)?;
    let image_picker =
        Picker::from_query_stdio().context("getting information for image display")?;
    let events = KeybindEvents::new()?;
    spawn::run_with_spawner(
        |spawner| run_app_inner(term, events, spawner, config, cache.clone(), image_picker),
        cancel,
        error_span!("jellyhaj"),
    )
    .await;
    Ok(())
}
