pub mod password;
pub mod quick_connect;
pub mod select;
pub mod server;

use std::{
    fs::{OpenOptions, create_dir_all, remove_file},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::Path,
    rc::Rc,
    sync::Arc,
};

use color_eyre::{
    Report, Result,
    eyre::{Context, OptionExt},
};
use jellyfin::{Authed, JellyfinClient};
use jellyhaj_core::{
    Config,
    state::{ClientOut, LoginState, Navigation, NextScreen},
    widgets::{
        shaded::widget::Erased,
        state::{StateStack, StateStackHandle, render_loop},
    },
};
use jellyhaj_fetch_view::make_nav_fetch;
use jellyhaj_widgets_core::{ContextRef, GetFromContext, async_task::UnboundedReceiver};
use keybinds::KeybindEvents;
use parking_lot::Mutex;
use ratatui::DefaultTerminal;
use spawn::Spawner;
use sqlx::SqliteConnection;
use stats_data::{Stats, StatsData};

type DB = Rc<tokio::sync::Mutex<SqliteConnection>>;

#[derive(Clone)]
pub struct LoginContext {
    pub config: Arc<Config>,
    pub cache: DB,
    pub spawner: Spawner,
    pub stats: Stats,
    pub state: Rc<StateStack>,
    pub original_login_state: LoginState,
}

impl ContextRef<Config> for LoginContext {
    fn as_ref(&self) -> &Config {
        &self.config
    }
}

impl ContextRef<DB> for LoginContext {
    fn as_ref(&self) -> &DB {
        &self.cache
    }
}

impl ContextRef<Spawner> for LoginContext {
    fn as_ref(&self) -> &Spawner {
        &self.spawner
    }
}

impl ContextRef<StatsData> for LoginContext {
    fn as_ref(&self) -> &StatsData {
        &self.stats
    }
}

impl ContextRef<StateStack> for LoginContext {
    fn as_ref(&self) -> &StateStack {
        &self.state
    }
}

#[allow(clippy::too_many_arguments)]
pub async fn login(
    config: Arc<Config>,
    db: DB,
    spawner: Spawner,
    stats: Stats,
    widget_creator_fn: fn(NextScreen, LoginContext) -> Erased,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    external: &mut UnboundedReceiver<NextScreen>,
) -> Option<JellyfinClient> {
    let (original_state, state_err) = match read_login_state(&config.login_file) {
        Ok(v) => (v, None),
        Err(e) => (LoginState::default(), Some(e)),
    };
    let state_stack = StateStackHandle::new();
    let cx = LoginContext {
        config,
        cache: db,
        spawner,
        stats,
        state: state_stack.clone(),
        original_login_state: original_state,
    };
    let widget_creator = {
        let cx = cx.clone();
        Rc::new(move |nav| widget_creator_fn(nav, cx.clone()))
    };
    let out = Arc::new(Mutex::new(None));
    let initial = if let Some(e) = state_err {
        cx.state.push(
            widget_creator_fn(
                NextScreen::SelectServer {
                    state: cx.original_login_state.clone(),
                    out: out.clone(),
                },
                cx.clone(),
            ),
            widget_creator.clone(),
        );
        NextScreen::Error(e)
    } else {
        NextScreen::SelectServer {
            state: cx.original_login_state.clone(),
            out: out.clone(),
        }
    };
    render_loop(initial, widget_creator, &cx.state, term, events, external).await;
    out.lock().take()
}

fn read_login_state(path: &Path) -> Result<LoginState> {
    if path.exists() {
        let content = std::fs::read_to_string(path).context("reading login file")?;
        toml::from_str(&content).context("reading login state")
    } else {
        Ok(LoginState::default())
    }
}

async fn store_login_result(
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient,
    server_id: String,
    db: DB,
    config: Arc<Config>,
    original_login_state: LoginState,
) -> Result<Navigation> {
    if config.store_access_token {
        let token = client.get_auth().token();
        sqlx::query!(
            "insert into creds (access_token, server_id) values (?,?)",
            token,
            server_id
        )
        .execute(&mut *db.lock().await)
        .await
        .context("storing credentials in cache")?;
    }
    if state != original_login_state {
        tokio::task::spawn_blocking(move || {
            create_dir_all(
                config
                    .login_file
                    .parent()
                    .ok_or_eyre("login info path has no parent")?,
            )
            .context("creating login info parent dir")?;
            if config.login_file.exists() {
                remove_file(&config.login_file).context("removing old login file")?;
            }
            OpenOptions::new()
                .create(true)
                .write(true)
                .truncate(true)
                .mode(0o0600)
                .open(&config.login_file)
                .context("opening login info")?
                .write_all(
                    toml::to_string_pretty(&state)
                        .context("serializing login info")?
                        .as_bytes(),
                )
                .context("writing out new login info")?;
            Ok::<(), Report>(())
        })
        .await
        .context("storing login info to file")??;
    }
    *out.lock() = Some(client);
    Ok(Navigation::Exit)
}

pub fn render_auth_finished(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient,
    server_id: String,
) -> Erased {
    let fut = store_login_result(
        state,
        out,
        client,
        server_id,
        cx.cache.clone(),
        cx.config.clone(),
        cx.original_login_state.clone(),
    );
    make_nav_fetch(cx, "Storing login information", fut)
}

pub fn render_logout(
    cx: impl ContextRef<JellyfinClient> + ContextRef<Config> + ContextRef<Spawner> + 'static,
) -> Erased {
    let jellyfin = JellyfinClient::get_ref(&cx).clone();
    let fut = async move {
        jellyfin.delete_current_api_key().await?;
        Ok(Navigation::Exit)
    };
    make_nav_fetch(cx, "Logout", fut)
}
