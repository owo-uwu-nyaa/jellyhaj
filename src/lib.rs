use std::{path::PathBuf, sync::Arc};

use color_eyre::{
    Result,
    eyre::{Context, OptionExt},
};
use config::{Config, init_config};
use jellyhaj_core::{
    context::TuiContext,
    render::{
        Erased, RenderStopRes, RunResult, StateStack, StateValue, render_widget, render_widget_stop,
    },
    state::{Navigation, NextScreen},
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

fn make_screen(screen: NextScreen, cx: TuiContext) -> Erased {
    match screen {
        NextScreen::LoadHomeScreen => jellyhaj_home_screen_view::make_fetch_home_screen(cx),
        NextScreen::HomeScreen {
            cont,
            next_up,
            libraries,
            library_latest,
        } => jellyhaj_home_screen_view::render_home_screen(
            cx,
            cont,
            next_up,
            libraries,
            library_latest,
        ),
        NextScreen::LoadUserView(user_view) => {
            jellyhaj_library_view::render_fetch_user_view(cx, user_view)
        }
        NextScreen::UserView { view, items } => {
            jellyhaj_library_view::render_user_view(cx, view, items)
        }
        NextScreen::FetchPlay(load_play) => jellyhaj_player_view::render_fetch_play(cx, load_play),
        NextScreen::Play { items, index } => jellyhaj_player_view::render_play(cx, items, index),
        NextScreen::Error(report) => jellyhaj_error_view::render_error(cx, &report),
        NextScreen::ItemDetails(media_item) => {
            jellyhaj_item_details_view::render_item_details(cx, media_item)
        }
        NextScreen::ItemListDetails(media_item, media_items) => {
            jellyhaj_item_details_view::render_item_list_details(cx, media_item, media_items)
        }
        NextScreen::FetchItemListDetails(media_item) => {
            jellyhaj_item_details_view::render_fetch_item_list(cx, media_item)
        }
        NextScreen::FetchItemListDetailsRef(id) => {
            jellyhaj_item_details_view::render_fetch_item_list_ref(cx, id)
        }
        NextScreen::FetchItemDetails(item) => {
            jellyhaj_item_details_view::render_fetch_episode(cx, item)
        }
        NextScreen::RefreshItem(id) => jellyhaj_refresh_item_view::render_refresh_item_form(cx, id),
        NextScreen::DoRefreshItem { id, query } => {
            jellyhaj_refresh_item_view::render_do_refresh_item(cx, id, query)
        }
        NextScreen::Stats => jellyhaj_stats_view::render_stats(cx),
        NextScreen::Logs => jellyhaj_log_view::render_log(cx),
        NextScreen::Inspect => jellyhaj_inspect_view::render_inspect(cx),
        NextScreen::QuickConnect => jellyhaj_quick_connect_view::make_quick_connect(cx),
        NextScreen::QuickConnectAuth(code) => {
            jellyhaj_quick_connect_view::make_quick_connect_auth(cx, code)
        }
    }
}

#[instrument(skip_all, level = "debug")]
async fn run_state(term: &mut DefaultTerminal, events: &mut KeybindEvents, cx: TuiContext) {
    let mut top: Option<NextScreen> = None;
    let widget_creator = {
        let cx = cx.clone();
        Arc::new(move |next| make_screen(next, cx.clone()))
    };
    info!("reached main application loop");
    loop {
        let mut widget = if let Some(top) = top.take() {
            debug!("running top next screen");
            make_screen(top, cx.clone())
        } else {
            match cx.state.pop() {
                StateValue::Suspended(suspended) => {
                    debug!("resuming suspended widget: {}", suspended.name);
                    match suspended.get_widget().await {
                        RunResult::Cont(erased_widget) => erased_widget,
                        RunResult::Empty => continue,
                        RunResult::Exit => break,
                    }
                }
                StateValue::Empty => {
                    debug!("defaulting to displaying home screen");
                    jellyhaj_home_screen_view::make_fetch_home_screen(cx.clone())
                }
                StateValue::WithoutTui(without_tui) => {
                    if let Err(e) = jellyhaj_core::term::run_without(without_tui).await {
                        jellyhaj_error_view::render_error(cx.clone(), &e)
                    } else {
                        continue;
                    }
                }
            }
        };
        match render_widget(widget.as_mut(), events, term).await.into() {
            Navigation::Push(next) => {
                cx.state.push(widget, widget_creator.clone());
                top = Some(next);
            }
            Navigation::PopContext => {
                match render_widget_stop(widget.as_mut(), events, term).await {
                    RenderStopRes::Ok => {}
                    RenderStopRes::Exit => break,
                }
            }
            Navigation::Replace(next) => {
                match render_widget_stop(widget.as_mut(), events, term).await {
                    RenderStopRes::Ok => top = Some(next),
                    RenderStopRes::Exit => break,
                }
            }
            Navigation::Exit => break,
            Navigation::PushWithoutTui(without_tui) => {
                if let Err(e) = jellyhaj_core::term::run_without(without_tui).await {
                    top = Some(NextScreen::Error(e));
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
    if let Some(jellyfin) = Box::pin(jellyhaj_login_view::login(
        clap::crate_name!(),
        clap::crate_version!(),
        &mut term,
        &mut events,
        spawner.clone(),
        config.clone(),
        &cache,
    ))
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
                stats: Arc::default(),
                spawn: spawner,
                state: Arc::new(StateStack::new()),
            },
        )
        .await;
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
