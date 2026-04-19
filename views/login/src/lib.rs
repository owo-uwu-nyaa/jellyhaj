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
    Report, Result,
    eyre::{Context, OptionExt, eyre},
};
use config::LoginInfo;
use jellyfin::{Auth, ClientInfo, JellyfinClient, NoAuth};
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
    info: &mut LoginInfo,
    changed: &mut bool,
    error: Report,
    events: &mut KeybindEvents,
    spawner: Spawner,
    config: Arc<Config>,
) -> Result<bool> {
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
        WidgetResult::Exit => Ok(false),
        WidgetResult::Ok(LoginResult::Quit) | WidgetResult::Pop => {
            render_widget_stop(widget.as_mut(), events, term).await;
            Ok(false)
        }
        WidgetResult::Ok(LoginResult::Data {
            server_url,
            username,
            password,
        }) => {
            let stop_res = render_widget_stop(widget.as_mut(), events, term).await;
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
            Ok(stop_res != RenderStopRes::Exit)
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
) -> Result<()> {
    let widget = MapperWidget::<Fetch, _, Fetch>::new(KeybindWidget::new(
        Loading::new(Cow::Borrowed("Connecting to Server")),
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
