use std::{
    fmt::Debug,
    fs::{OpenOptions, create_dir_all},
    io::Write,
    ops::ControlFlow,
    os::unix::fs::OpenOptionsExt,
    sync::Arc, time::Duration,
};

use color_eyre::{
    Report, Result, Section, SectionExt,
    eyre::{Context, OptionExt, eyre},
};
use config::LoginInfo;
use jellyfin::{
    Auth, ClientInfo, JellyfinClient, NoAuth, auth::UniqueId, quick_connect::QuickConnectStatus,
};
use jellyhaj_core::{
    CommandMapper, Config,
    context::ContextRef,
    keybinds::LoadingCommand,
    render::{RenderStopRes, WidgetResult, make_new_erased, render_widget, render_widget_stop},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_loading_widget::{AdvanceLoadingScreen, Loading};
use jellyhaj_login_widget::{
    LoginResult, LoginType, LoginWidget, QuickConectAction, QuickConnectWidget, Quit,
};
use jellyhaj_widgets_core::mapper::MapperWidget;
use keybinds::KeybindEvents;
use ratatui::DefaultTerminal;
use spawn::Spawner;
use sqlx::SqliteConnection;
use std::result::Result as StdResult;
use tokio::{select, time::sleep};
use tracing::{info, instrument};
struct LoginContext {
    config: Arc<Config>,
    spawner: Spawner,
}

impl ContextRef<Config> for LoginContext {
    fn as_ref(&self) -> &Config {
        &self.config
    }
}

impl ContextRef<Spawner> for LoginContext {
    fn as_ref(&self) -> &Spawner {
        &self.spawner
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn edit_login_info(
    term: &mut DefaultTerminal,
    info: &LoginInfo,
    error: Report,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
) -> Result<Option<LoginType>> {
    tracing::error!("error during login: {error:?}");
    let widget = LoginWidget::new(
        info.server_url.clone(),
        info.username.clone(),
        info.password.clone(),
        info.password_cmd.is_some(),
        error,
        &config,
    );
    let cx = LoginContext { config, spawner };
    let mut widget = make_new_erased(cx, widget);

    match render_widget(widget.as_mut(), events, term).await {
        WidgetResult::Exit => Ok(None),
        WidgetResult::Ok(LoginResult::Quit) | WidgetResult::Pop => {
            render_widget_stop(widget.as_mut(), events, term).await;
            Ok(None)
        }
        WidgetResult::Ok(LoginResult::Login(login)) => {
            let stop_res = render_widget_stop(widget.as_mut(), events, term).await;
            if stop_res != RenderStopRes::Exit {
                Ok(Some(login))
            } else {
                Ok(None)
            }
        }
        WidgetResult::Err(report) => Err(report),
    }
}

struct LoadingMapper;

impl CommandMapper<LoadingCommand> for LoadingMapper {
    type A = AdvanceLoadingScreen;

    fn map(
        &self,
        command: LoadingCommand,
    ) -> std::ops::ControlFlow<jellyhaj_core::state::Navigation, Self::A> {
        match command {
            LoadingCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            LoadingCommand::Global(g) => ControlFlow::Break(g.into()),
        }
    }
}

struct Fetch;

impl<I: Debug + 'static> jellyhaj_widgets_core::mapper::ResultMapper<I> for Fetch {
    type R = I;

    fn map(res: I) -> Result<Option<Self::R>> {
        Ok(Some(res))
    }
}
impl jellyhaj_widgets_core::mapper::Named for Fetch {
    const NAME: &str = "fetch";
}

#[allow(clippy::too_many_arguments)]
async fn render_fetch(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
    message: &'static str,
) -> Result<()> {
    let widget = MapperWidget::<Fetch, _, Fetch>::new(KeybindWidget::new(
        Loading::new(message),
        config.keybinds.fetch.clone(),
        LoadingMapper,
    ));
    let cx = LoginContext { config, spawner };
    let mut widget = make_new_erased(cx, widget);
    match render_widget(widget.as_mut(), events, term).await {
        WidgetResult::Ok(ControlFlow::Break(_)) | WidgetResult::Pop => {
            render_widget_stop(widget.as_mut(), events, term).await;
            Ok(())
        }
        WidgetResult::Err(report) => {
            render_widget_stop(widget.as_mut(), events, term).await;
            Err(report)
        }
        WidgetResult::Exit => Ok(()),
    }
}

struct QuickConnectMapper;

impl CommandMapper<LoadingCommand> for QuickConnectMapper {
    type A = QuickConectAction;

    fn map(&self, command: LoadingCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            LoadingCommand::Quit => ControlFlow::Continue(QuickConectAction::Quit),
            LoadingCommand::Global(global_command) => ControlFlow::Break(global_command.into()),
        }
    }
}

async fn render_quick_connect(
    code: String,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
) -> Result<Option<Quit>> {
    let widget = KeybindWidget::new(
        QuickConnectWidget::new(code),
        config.keybinds.fetch.clone(),
        QuickConnectMapper,
    );
    let cx = LoginContext { config, spawner };
    let mut widget = make_new_erased(cx, widget);
    loop {
        break match render_widget(widget.as_mut(), events, term).await {
            WidgetResult::Pop | WidgetResult::Ok(ControlFlow::Continue(_)) => {
                render_widget_stop(widget.as_mut(), events, term).await;
                Ok(Some(Quit))
            }
            WidgetResult::Ok(_) => {
                continue;
            }
            WidgetResult::Err(report) => {
                render_widget_stop(widget.as_mut(), events, term).await;
                Err(report)
            }
            WidgetResult::Exit => Ok(None),
        };
    }
}

async fn with_render<F: Future>(
    f: F,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
    message: &'static str,
) -> Result<Option<F::Output>> {
    select! {
        res = f => {
            Ok(Some(res))
        }
        res = render_fetch(
            term, events, spawner, config, message
        ) => {
            if let Err(e) = res{
                Err(e)
            }else{
                Ok(None)
            }
        }
    }
}

macro_rules! rendered {
    ($val:expr) => {
        match $val.await {
            Err(e) => return Err(e),
            Ok(None) => return Ok(None),
            Ok(Some(v)) => v,
        }
    };
}

#[allow(clippy::too_many_arguments)]
pub async fn login_loop(
    name: &'static str,
    version: &'static str,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
    db: &mut SqliteConnection,
    device_name: String,
    login_info: &mut LoginInfo,
    unique_id: UniqueId,
    changed: &mut bool,
) -> Result<Option<JellyfinClient<Auth>>> {
    let mut current_client;
    if login_info.server_url.is_empty() {
        current_client = None;
    } else if let Some(creds) = get_stored_creds(db, config.store_access_token).await?
        && let Ok(client) = JellyfinClient::new_auth_key(
            login_info.server_url.clone(),
            ClientInfo {
                name: name.into(),
                version: version.into(),
            },
            device_name.clone(),
            creds.access_token,
            unique_id,
            config.concurrent_jellyfin_connections.into(),
        )
    {
        match rendered!(with_render(
            client.get_self(),
            term,
            events,
            spawner.clone(),
            config.clone(),
            "Testing stored credentials"
        )) {
            Ok(client) => {
                info!("stored credentials still worked");
                return Ok(Some(client));
            }
            Err((client, _)) => current_client = Some(client.without_auth()),
        }
    } else {
        current_client = Some(
            JellyfinClient::new(
                login_info.server_url.clone(),
                ClientInfo {
                    name: name.into(),
                    version: version.into(),
                },
                device_name.clone(),
                unique_id,
                config.concurrent_jellyfin_connections.into(),
            )
            .with_section(|| {
                "1. Delete the file\n2. Try again and fill in the login information".header("Tipp:")
            })?,
        );
    }
    let mut error;
    if let Some(client) = current_client {
        match rendered!(with_render(
            jellyfin_login_pw(client, login_info),
            term,
            events,
            spawner.clone(),
            config.clone(),
            "Testing stored credentials"
        )) {
            Ok(client) => {
                info!("logged in with stored credentials");
                return Ok(Some(client));
            }
            Err((client, e)) => {
                current_client = Some(client);
                error = e
            }
        }
    } else {
        error = eyre!("Server url is empty");
    }

    loop {
        let login_action;
        match rendered!(edit_login_info(
            term,
            login_info,
            error,
            events,
            spawner.clone(),
            config.clone()
        )) {
            LoginType::Password {
                server_url,
                username,
                password,
            } => {
                login_action = LoginKind::PW;
                if server_url != login_info.server_url {
                    if server_url.is_empty() {
                        current_client = None;
                    } else {
                        current_client = match JellyfinClient::new(
                            &server_url,
                            ClientInfo {
                                name: name.into(),
                                version: version.into(),
                            },
                            device_name.clone(),
                            unique_id,
                            config.concurrent_jellyfin_connections.into(),
                        ) {
                            Ok(v) => Some(v),
                            Err(e) => {
                                error = e;
                                continue;
                            }
                        };
                    }
                    login_info.server_url = server_url;
                    *changed = true;
                }
                if username != login_info.username {
                    login_info.username = username;
                    *changed = true;
                }
                if password != login_info.password {
                    login_info.password = password;
                    *changed = true;
                }
            }
            LoginType::QuickConnect { server_url } => {
                login_action = LoginKind::QuickConnect;
                if server_url != login_info.server_url {
                    current_client = match JellyfinClient::new(
                        &server_url,
                        ClientInfo {
                            name: name.into(),
                            version: version.into(),
                        },
                        device_name.clone(),
                        unique_id,
                        config.concurrent_jellyfin_connections.into(),
                    ) {
                        Ok(v) => Some(v),
                        Err(e) => {
                            error = e;
                            continue;
                        }
                    };
                    login_info.server_url = server_url;
                    *changed = true;
                }
            }
        }
        if let Some(client) = current_client {
            match rendered!(jellyfin_login(
                client,
                term,
                events,
                spawner.clone(),
                config.clone(),
                login_action,
                login_info
            )) {
                Ok(c) => return Ok(Some(c)),
                Err((c, e)) => {
                    current_client = Some(c);
                    error = e;
                }
            }
        } else {
            error = eyre!("Server url is empty");
        }
    }
}

enum LoginKind {
    PW,
    QuickConnect,
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
pub async fn login(
    name: &'static str,
    version: &'static str,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
    db: &tokio::sync::Mutex<SqliteConnection>,
) -> Result<Option<JellyfinClient<Auth>>> {
    let mut db = db.lock().await;
    let device_name = whoami::hostname()
        .ok()
        .unwrap_or_else(|| "unknown".to_owned());

    let mut login_info = if config.login_file.exists() {
        std::fs::read_to_string(&config.login_file)
            .context("reading login info file")
            .and_then(|config| {
                toml::from_str::<LoginInfo>(&config)
                    .context("parsing login info")
                    .with_section(|| {
                        "1. Delete the file\n2. Try again and fill in the login information"
                            .header("Tipp:")
                    })
            })
            .with_section(|| {
                config
                    .login_file
                    .display()
                    .to_string()
                    .header("File location:")
            })?
    } else {
        LoginInfo {
            server_url: String::new(),
            username: String::new(),
            password: String::new(),
            password_cmd: None,
        }
    };
    let mut changed = false;
    let unique_id = get_unique(&mut db).await?;
    let client = rendered!(login_loop(
        name,
        version,
        term,
        events,
        spawner,
        config.clone(),
        &mut db,
        device_name,
        &mut login_info,
        unique_id,
        &mut changed
    ));

    if changed {
        create_dir_all(
            config
                .login_file
                .parent()
                .ok_or_eyre("login info path has no parent")?,
        )
        .context("creating login info parent dir")?;
        OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .mode(0o0600)
            .open(&config.login_file)
            .context("opening login info")?
            .write_all(
                toml::to_string_pretty(&login_info)
                    .context("serializing login info")?
                    .as_bytes(),
            )
            .context("writing out new login info")?;
    }
    if config.store_access_token {
        store_creds(&mut db, &client.get_auth().access_token).await?
    }
    Ok(Some(client))
}

async fn jellyfin_login(
    client: JellyfinClient<NoAuth>,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
    kind: LoginKind,
    info: &LoginInfo,
) -> Result<Option<StdResult<JellyfinClient, (JellyfinClient<NoAuth>, Report)>>> {
    match kind {
        LoginKind::PW => {
            with_render(
                jellyfin_login_pw(client, info),
                term,
                events,
                spawner,
                config,
                "Logging in",
            )
            .await
        }
        LoginKind::QuickConnect => {
            jellyfin_login_quick_connect(client, term, events, spawner, config).await
        }
    }
}

async fn jellyfin_login_pw(
    client: JellyfinClient<NoAuth>,
    info: &LoginInfo,
) -> StdResult<JellyfinClient<Auth>, (JellyfinClient<NoAuth>, Report)> {
    info!("connecting to server");
    let password = match info.get_password().await {
        Ok(v) => v,
        Err(e) => return Err((client, e)),
    };
    let client = match client.auth_user_name(&info.username, password).await {
        Ok(v) => v,
        Err((client, e)) => return Err((client, e)),
    };
    Ok(client)
}

async fn jellyfin_login_quick_connect(
    client: JellyfinClient<NoAuth>,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
) -> Result<Option<StdResult<JellyfinClient, (JellyfinClient<NoAuth>, Report)>>> {
    let available = async { client.quick_connect_enabled().await?.deserialize().await };
    match rendered!(with_render(
        available,
        term,
        events,
        spawner.clone(),
        config.clone(),
        "Getting Quick Connect Status"
    )) {
        Ok(true) => {}
        Ok(false) => {
            return Ok(Some(Err((
                client,
                eyre!("This server does not support quick connect!"),
            ))));
        }
        Err(e) => {
            return Ok(Some(Err((
                client,
                e.suggestion("check if the server url is correct"),
            ))));
        }
    }
    let start_quick_connect = async { client.initiate_quick_connect().await?.deserialize().await };
    let mut quick_connect_status = match rendered!(with_render(
        start_quick_connect,
        term,
        events,
        spawner.clone(),
        config.clone(),
        "Requesting quick connect"
    )) {
        Ok(v) => v,
        Err(e) => return Ok(Some(Err((client, e.wrap_err("Starting quick connect"))))),
    };
    quick_connect_status = match rendered!(poll_quick_connect(
        quick_connect_status.secret,
        quick_connect_status.code,
        term,
        events,
        spawner.clone(),
        config.clone(),
        &client
    )) {
        Ok(v) => v,
        Err(e) => return Ok(Some(Err((client, e)))),
    };
    Ok(Some(
        client
            .auth_quick_connect(&quick_connect_status.secret)
            .await,
    ))
}

async fn poll_quick_connect(
    secret: String,
    code: String,
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
    client: &JellyfinClient<NoAuth>,
) -> Result<Option<Result<QuickConnectStatus>>> {
    let status = async {
        loop {
            sleep(Duration::from_secs(2)).await;
            let status = client
                .get_quick_connect_status(&secret)
                .await?
                .deserialize()
                .await?;
            if status.authenticated {
                break Ok(status);
            }
        }
    };
    select! {
        res = render_quick_connect(code, term, events, spawner, config) => {
            match res{
                Err(e) => Err(e),
                Ok(None) => Ok(None),
                Ok(Some(_)) => Ok(Some(Err(eyre!("Cancelled quick connect")))),
            }
        }
        res = status => {
            Ok(Some(res))
        }
    }
}

async fn get_unique(db: &mut SqliteConnection) -> Result<UniqueId> {
    let val = sqlx::query_scalar!("select id from unique_id")
        .fetch_optional(&mut *db)
        .await?;
    if let Some(v) = val.and_then(|v| <[u8; 64]>::try_from(v).ok().map(UniqueId)) {
        Ok(v)
    } else {
        let id = UniqueId::generate_new()?;
        let id_val = id.0.as_slice();
        sqlx::query!("insert into unique_id (id) values (?)", id_val)
            .execute(db)
            .await?;
        Ok(id)
    }
}

struct StoredCreds {
    access_token: String,
}

async fn get_stored_creds(db: &mut SqliteConnection, store: bool) -> Result<Option<StoredCreds>> {
    if store {
        sqlx::query_as!(StoredCreds, "select access_token from creds")
            .fetch_optional(db)
            .await
            .context("getting stored credentials")
    } else {
        sqlx::query!("delete from creds")
            .execute(db)
            .await
            .context("clearing stored credentials")?;
        Ok(None)
    }
}

async fn store_creds(db: &mut SqliteConnection, token: &str) -> Result<()> {
    sqlx::query!("insert into creds (access_token) values (?)", token)
        .execute(db)
        .await
        .context("storing credentials in cache")?;
    Ok(())
}
