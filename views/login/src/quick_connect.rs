use std::{ops::ControlFlow, time::Duration};

use color_eyre::eyre::Context;
use config::keybind_defs::LoadingCommand;
use jellyfin::{
    JellyfinClient, NoAuth, connect::JsonResponseHelper, quick_connect::QuickConnectStatus,
};
use jellyhaj_core::{
    CommandMapper,
    state::{ClientOut, LoginState, Navigation, NextScreen},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_login_widget::quick_connect::{QuickConectAction, QuickConnectWidget};
use jellyhaj_widgets_core::{
    Result,
    outer::{Named, OuterWidget},
};

use crate::LoginContext;

async fn start_quick_connect(
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    server_id: String,
) -> Result<NextScreen> {
    let quick_connect: QuickConnectStatus = client.initiate_quick_connect().deserialize().await?;
    Ok(NextScreen::AuthQuickConnectWait {
        state,
        out,
        client,
        secret: quick_connect.secret,
        code: quick_connect.code,
        server_id,
    })
}

pub fn render_auth_quick_connect_fetch(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    server_id: String,
) -> Erased {
    let fut = start_quick_connect(state, out, client, server_id);
    make_fetch(cx, "Initiating Quick Connect", fut)
}

async fn wait_quick_connect(
    client: JellyfinClient<NoAuth>,
    secret: String,
) -> Result<QuickConectAction> {
    while !client
        .get_quick_connect_status(&secret)
        .deserialize()
        .await
        .context("fetching quick connect status")?
        .authenticated
    {
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    match client.auth_quick_connect(&secret).await {
        Ok(client) => Ok(QuickConectAction::Finished { client }),
        Err((_, e)) => Err(e.wrap_err("authenticating through quick connect")),
    }
}

struct Helper;

impl CommandMapper<LoadingCommand> for Helper {
    type A = QuickConectAction;

    fn map(&self, command: LoadingCommand) -> ControlFlow<Navigation, Self::A> {
        match command {
            LoadingCommand::Quit => ControlFlow::Break(Navigation::PopContext),
            LoadingCommand::Global(global_command) => ControlFlow::Break(global_command.into()),
        }
    }
}
impl Named for Helper {
    const NAME: &str = "quick-connect-wait";
}

pub fn render_auth_quick_connect_wait(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    secret: String,
    code: String,
    server_id: String,
) -> Erased {
    let widget = QuickConnectWidget::new(
        code,
        wait_quick_connect(client, secret),
        state,
        out,
        server_id,
    );
    let widget = KeybindWidget::new(widget, cx.config.keybinds.fetch.clone(), Helper);
    let widget = OuterWidget::<Helper, _>::new(widget);
    make_new_erased(cx, widget)
}
