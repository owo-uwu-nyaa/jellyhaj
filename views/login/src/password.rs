use color_eyre::eyre::{Context, eyre};
use jellyfin::{JellyfinClient, NoAuth};
use jellyhaj_core::{
    state::{ClientOut, LoginState, NextScreen},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_form_widget::form::{FormCommandMapper, FormData};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_login_widget::password::{
    PasswordData, PasswordDataAction, PasswordDataSelection, PasswordWidget,
};
use jellyhaj_widgets_core::Result;
use tokio::process::Command;

use crate::LoginContext;

pub fn render_password(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    server_id: String,
) -> Erased {
    let widget = PasswordData::new(
        state.username.clone(),
        state.password.clone(),
        &state.passwort_cmd,
    )
    .make_with(PasswordDataSelection::Username(()));
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<PasswordDataAction>::default(),
    );
    let widget = PasswordWidget::new(widget, out, state, client, server_id);
    make_new_erased(cx, widget)
}

async fn password_fetch(
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    server_id: String,
) -> Result<NextScreen> {
    let pw = if state.passwort_cmd.is_empty() {
        state.password.clone()
    } else {
        let mut command = Command::new(state.passwort_cmd.first().expect("already checked"));
        for arg in &state.passwort_cmd[1..] {
            command.arg(arg);
        }
        let output = command
            .kill_on_drop(true)
            .output()
            .await
            .context("Executing password cmd failed")?;
        if output.status.success() {
            String::from_utf8(output.stdout)
                .context("password cmd output is not utf-8")?
                .trim()
                .to_string()
        } else {
            return Err(eyre!(
                "command failed with:\n{}",
                String::from_utf8(output.stderr)
                    .context("password cmd error output is not utf-8")?
            ));
        }
    };
    match client.auth_user_name(&state.username, &pw).await {
        Ok(client) => Ok(NextScreen::AuthFinished {
            state,
            out,
            client,
            server_id,
        }),
        Err((_, e)) => Err(e.wrap_err("Logging in")),
    }
}

pub fn render_password_fetch(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    client: JellyfinClient<NoAuth>,
    server_id: String,
) -> Erased {
    make_fetch(
        cx,
        "Logging in",
        password_fetch(state, out, client, server_id),
    )
}
