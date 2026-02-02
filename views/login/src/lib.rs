use std::{
    borrow::Cow,
    fs::{OpenOptions, create_dir_all},
    io::Write,
    os::unix::fs::OpenOptionsExt,
};

use color_eyre::{
    Report, Result,
    eyre::{Context, OptionExt, eyre},
};
use jellyfin::{Auth, ClientInfo, JellyfinClient, NoAuth};
use jellyhaj_core::{
    Config,
    keybinds::{Keybinds, LoginInfoCommand},
};
use jellyhaj_fetch_view::render_fetch;
use jellyhaj_keybinds_widget::{CommandAction, KeybindWidget, MappedCommand};
use jellyhaj_login_widget::{LoginAction, LoginInfo, LoginSelection, LoginWidget};
use jellyhaj_render_widgets::TermExt;
use keybinds::KeybindEvents;
use ratatui::DefaultTerminal;
use spawn::Spawner;
use tokio::select;
use tracing::{info, instrument};

struct Quit;

#[allow(clippy::too_many_arguments)]
#[instrument(skip_all)]
async fn edit_login_info(
    term: &mut DefaultTerminal,
    spawn: Spawner,
    info: &mut LoginInfo,
    changed: &mut bool,
    error: Report,
    events: &mut KeybindEvents,
    keybinds: &Keybinds,
    help_prefixes: &[String],
) -> Result<bool> {
    let selection = if info.server_url.is_empty() {
        LoginSelection::Server
    } else {
        LoginSelection::Password
    };
    let error = error.to_string();

    let mut widget = KeybindWidget::new(
        LoginWidget::new(info, selection, error),
        help_prefixes,
        keybinds.login_info.clone(),
        |login| match login {
            LoginInfoCommand::Delete => MappedCommand::Down(LoginAction::Delete),
            LoginInfoCommand::Submit => MappedCommand::Down(LoginAction::Submit),
            LoginInfoCommand::Next => MappedCommand::Down(LoginAction::Next),
            LoginInfoCommand::Prev => MappedCommand::Down(LoginAction::Prev),
            LoginInfoCommand::Quit => MappedCommand::Up(Quit),
        },
    );
    Ok(match term.render(&mut widget, events, spawn).await? {
        CommandAction::Up(Quit) | CommandAction::Exit => false,
        CommandAction::Action(e) => {
            *changed = e.changed;
            true
        }
    })
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
                &config.keybinds,
                &config.help_prefixes,
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
                    "Connecting to Server",
                    events,
                    config.keybinds.fetch.clone(),
                    term,
                    &config.help_prefixes,
                    spawn.clone(),
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
