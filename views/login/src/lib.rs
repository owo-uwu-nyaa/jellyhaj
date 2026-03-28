use std::{
    borrow::Cow,
    fs::{OpenOptions, create_dir_all},
    io::Write,
    ops::ControlFlow,
    os::unix::fs::OpenOptionsExt,
    sync::Arc,
};

use color_eyre::{
    Report, Result,
    eyre::{Context, OptionExt, eyre},
};
use config::LoginInfo;
use jellyfin::{Auth, ClientInfo, JellyfinClient, NoAuth};
use jellyhaj_core::{
    CommandMapper, Config,
    context::ContextRef,
    keybinds::LoadingCommand,
    render::{RenderResult, render_widget_bare},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_loading_widget::{AdvanceLoadingScreen, Loading};
use jellyhaj_login_widget::{LoginResult, LoginWidget};
use keybinds::KeybindEvents;
use ratatui::DefaultTerminal;
use spawn::Spawner;
use tokio::select;
use tracing::{info, instrument};

struct LoginContext {
    config: Arc<Config>,
}

impl ContextRef<Config> for LoginContext {
    fn get_ref(&self) -> &Config {
        &self.config
    }
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn edit_login_info(
    term: &mut DefaultTerminal,
    info: &mut LoginInfo,
    changed: &mut bool,
    error: Report,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
) -> Result<bool> {
    let error = error.to_string();
    let cx = LoginContext { config };

    let widget = LoginWidget::new(
        info.server_url.clone(),
        info.username.clone(),
        info.password.clone(),
        info.password_cmd.is_some(),
        error,
        &cx,
    );
    match render_widget_bare(term, events, spawner, widget, cx).await {
        RenderResult::Ok((LoginResult::Quit, _)) => Ok(false),
        RenderResult::Ok((
            LoginResult::Data {
                server_url,
                username,
                password,
            },
            _,
        )) => {
            if server_url != info.server_url {
                info.server_url = server_url;
                *changed = true;
            }
            if username != info.username {
                info.username = username;
                *changed = true;
            }
            if password != info.password {
                info.password = password;
                *changed = true;
            }
            Ok(true)
        }
        RenderResult::Err(report) => Err(report),
        RenderResult::Exit => Ok(false),
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

#[allow(clippy::too_many_arguments)]
async fn render_fetch(
    term: &mut DefaultTerminal,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
) -> Result<()> {
    let widget = KeybindWidget::new(
        Loading::new(Cow::Borrowed("Connecting to Server")),
        config.keybinds.fetch.clone(),
        LoadingMapper,
    );
    let cx = LoginContext { config };
    match render_widget_bare(term, events, spawner, widget, cx).await {
        RenderResult::Ok((ControlFlow::Break(_), _)) => Ok(()),
        RenderResult::Err(report) => Err(report),
        RenderResult::Exit => Ok(()),
    }
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
) -> Result<Option<JellyfinClient<Auth>>> {
    let mut login_info: LoginInfo;
    let mut error: Option<Report>;
    match std::fs::read_to_string(&config.login_file)
        .context("reading login info file")
        .and_then(|config| toml::from_str::<LoginInfo>(&config).context("parsing login info"))
    {
        Ok(info) => {
            login_info = info;
            error = None;
        }
        Err(e) => {
            login_info = LoginInfo {
                server_url: String::new(),
                username: String::new(),
                password: String::new(),
                password_cmd: None,
            };
            error = Some(e);
        }
    }
    let mut info_changed = false;
    let device_name: Cow<'static, str> = whoami::hostname()
        .ok()
        .map(|v| v.into())
        .unwrap_or_else(|| "unknown".into());

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
            config.concurrent_jellyfin_connections.into(),
        ) {
            Ok(client) => client,
            Err(e) => {
                error = Some(e);
                continue;
            }
        };

        select! {
            r = render_fetch(term, events, spawner.clone(), config.clone())
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

async fn jellyfin_login(
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
