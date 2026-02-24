use std::{
    borrow::Cow,
    fs::{OpenOptions, create_dir_all},
    io::Write,
    ops::ControlFlow,
    os::unix::fs::OpenOptionsExt,
};

use color_eyre::{
    Report, Result,
    eyre::{Context, OptionExt, eyre},
};
use jellyfin::{Auth, ClientInfo, JellyfinClient, NoAuth};
use jellyhaj_core::{
    CommandMapper, Config,
    keybinds::LoadingCommand,
    render::{RenderResult, render_widget_bare},
    state::Navigation,
};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_loading_widget::{AdvanceLoadingScreen, Loading};
use jellyhaj_login_widget::{LoginResult, LoginWidget};
use keybinds::KeybindEvents;
use ratatui::DefaultTerminal;
use serde::{Deserialize, Serialize};
use spawn::Spawner;
use tokio::select;
use tracing::{info, instrument};

#[derive(Debug, Deserialize, Serialize)]
pub struct LoginInfo {
    pub server_url: String,
    pub username: String,
    pub password: String,
    pub password_cmd: Option<Vec<String>>,
}

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn edit_login_info(
    term: &mut DefaultTerminal,
    spawn: Spawner,
    info: &mut LoginInfo,
    changed: &mut bool,
    error: Report,
    events: &mut KeybindEvents,
    config: &Config,
) -> Result<bool> {
    let error = error.to_string();

    let widget = LoginWidget::new(
        info.server_url.clone(),
        info.username.clone(),
        info.password.clone(),
        info.password_cmd.is_some(),
        error,
        config,
    );
    match render_widget_bare(term, events, spawn, widget).await {
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
            LoadingCommand::Global(g) => ControlFlow::Break(g.into())
        }
    }
}

async fn render_fetch(
    term: &mut DefaultTerminal,
    spawn: Spawner,
    events: &mut KeybindEvents,
    config: &Config,
) -> Result<()> {
    let widget = KeybindWidget::new(
        Loading::new(Cow::Borrowed("Connecting to Server")),
        config.help_prefixes.clone(),
        config.keybinds.fetch.clone(),
        LoadingMapper,
    );
    match render_widget_bare(term, events, spawn, widget).await {
        RenderResult::Ok((ControlFlow::Break(_), _)) => Ok(()),
        RenderResult::Err(report) => Err(report),
        RenderResult::Exit => Ok(()),
    }
}

#[instrument(skip_all)]
pub async fn login(
    term: &mut DefaultTerminal,
    spawn: &Spawner,
    config: &Config,
    events: &mut KeybindEvents,
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
                spawn.clone(),
                &mut login_info,
                &mut info_changed,
                e,
                events,
                config,
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
        let client = match JellyfinClient::<NoAuth>::new(
            &login_info.server_url,
            ClientInfo {
                name: "jellyhaj".into(),
                version: "0.2.0".into(),
            },
            device_name.clone(),
        ) {
            Ok(client) => client,
            Err(e) => {
                error = Some(e);
                continue;
            }
        };

        select! {
            r = render_fetch(
                term,
                spawn.clone(),
                events,
                config,
            ) => {
                r?;
                return Ok(None);
            }
            v = jellyfin_login(
                client,
                &login_info.username,
                &login_info.password,
                login_info.password_cmd.as_deref(),
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
    username: &str,
    password: &str,
    password_cmd: Option<&[String]>,
) -> std::result::Result<JellyfinClient<Auth>, (JellyfinClient<NoAuth>, Report)> {
    info!("connecting to server");
    let password = if let Some(cmd) = password_cmd {
        match get_password_from_cmd(cmd).await {
            Ok(v) => v,
            Err(e) => return Err((client, e)),
        }
    } else {
        password.to_string()
    };
    let client = match client.auth_user_name(username, password).await {
        Ok(v) => v,
        Err((client, e)) => return Err((client, e)),
    };
    Ok(client)
}

async fn get_password_from_cmd(cmd: &[String]) -> Result<String> {
    let mut command = if let Some(cmd) = cmd.first() {
        tokio::process::Command::new(cmd)
    } else {
        return Err(eyre!("Password cmd is empty"));
    };
    for arg in cmd[1..].iter() {
        command.arg(arg);
    }
    let output = command
        .kill_on_drop(true)
        .output()
        .await
        .context("Executing password cmd failed")?;
    if output.status.success() {
        Ok(String::from_utf8(output.stdout)
            .context("password cmd output is not utf-8")?
            .trim()
            .to_string())
    } else {
        Err(eyre!(
            "command failed with:\n{}",
            String::from_utf8(output.stderr).context("password cmd error output is not utf-8")?
        ))
    }
}
