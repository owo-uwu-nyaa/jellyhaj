use std::{
    borrow::Cow,
    fmt::Debug,
    fs::{OpenOptions, create_dir_all},
    io::Write,
    ops::ControlFlow,
    os::unix::fs::OpenOptionsExt,
    sync::Arc,
};

use color_eyre::{
    Report, Result, Section, SectionExt,
    eyre::{Context, OptionExt, eyre},
};
use config::LoginInfo;
use jellyfin::{Auth, ClientInfo, JellyfinClient, NoAuth, auth::UniqueId};
use jellyhaj_core::{
    CommandMapper, Config,
    context::ContextRef,
    keybinds::LoadingCommand,
    render::{RenderStopRes, WidgetResult, make_new_erased, render_widget, render_widget_stop},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_loading_widget::{AdvanceLoadingScreen, Loading};
use jellyhaj_login_widget::{LoginResult, LoginWidget};
use jellyhaj_widgets_core::mapper::MapperWidget;
use keybinds::KeybindEvents;
use ratatui::DefaultTerminal;
use spawn::Spawner;
use sqlx::SqliteConnection;
use tokio::select;
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
) -> Result<Option<LoginInfo>> {
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
        WidgetResult::Ok(LoginResult::Data {
            server_url,
            username,
            password,
        }) => {
            let stop_res = render_widget_stop(widget.as_mut(), events, term).await;
            if stop_res != RenderStopRes::Exit {
                Ok(Some(LoginInfo {
                    server_url,
                    username,
                    password,
                    password_cmd: None,
                }))
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
) -> Result<Option<JellyfinClient<Auth>>> {
    let mut current_client;
    if let Some(creds) = get_stored_creds(db, config.store_access_token).await?
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
            Ok(client) => return Ok(Some(client)),
            Err((client, _)) => current_client = client.without_auth(),
        }
    } else {
        current_client = JellyfinClient::new(
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
        })?;
    }

    loop {
        let error;
        match rendered!(with_render(
            jellyfin_login_pw(current_client, login_info),
            term,
            events,
            spawner.clone(),
            config.clone(),
            "Testing stored credentials"
        )) {
            Ok(client) => return Ok(Some(client)),
            Err((client, e)) => {
                current_client = client;
                error = e
            }
        }
    }

    todo!()
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
    let mut login_info: LoginInfo;
    let mut error: Option<Report> = None;
    let mut info_changed = false;

    let device_name: Cow<'static, str> = whoami::hostname()
        .ok()
        .map(|v| v.into())
        .unwrap_or_else(|| "unknown".into());

    let mut login_info = std::fs::read_to_string(&config.login_file)
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
        })?;

    let client = loop {
        if let Some(e) = error.take() {
            tracing::error!("Error logging in: {e:?}");
            if !edit_login_info(
                term,
                &mut login_info,
                &mut info_changed,
                e,
                events,
                spawner.clone(),
                config.clone(),
            )
            .await
            .context("getting login information")?
            {
                return Ok(None);
            }
            if login_info.server_url.is_empty() {
                error = Some(eyre!("Server URI is empty"));
                continue;
            }
        }
        let client = match JellyfinClient::new(
            &login_info.server_url,
            ClientInfo {
                name: name.into(),
                version: version.into(),
            },
            device_name.clone(),
            get_unique(&mut db).await?,
            config.concurrent_jellyfin_connections.into(),
        ) {
            Ok(client) => client,
            Err(e) => {
                error = Some(e);
                continue;
            }
        };

        select! {
            r = render_fetch(term, events, spawner.clone(), config.clone(),"Connecting to server")
                => {
                r?;
                return Ok(None);
            }
            v = jellyfin_login(
                client,
                &login_info,
            ) => {
                match v {
                    Ok(v) => break v,
                    Err((_, e)) => {
                        error = Some(e.wrap_err("logging in"));
                    }
                }
            }
        }
    };
    if info_changed {
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
    Ok(Some(client))
}

async fn jellyfin_login_pw(
    client: JellyfinClient<NoAuth>,
    info: &LoginInfo,
) -> std::result::Result<JellyfinClient<Auth>, (JellyfinClient<NoAuth>, Report)> {
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
    user_name: String,
    access_token: String,
}

async fn get_stored_creds(db: &mut SqliteConnection, store: bool) -> Result<Option<StoredCreds>> {
    if store {
        sqlx::query_as!(StoredCreds, "select user_name, access_token from creds")
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

async fn store_creds(db: &mut SqliteConnection, user_name: &str, token: &str) -> Result<()> {
    sqlx::query!(
        "insert into creds (user_name, access_token) values (?,?)",
        user_name,
        token
    )
    .execute(db)
    .await
    .context("storing credentials in cache")?;
    Ok(())
}
