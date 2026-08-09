use std::sync::Arc;

use color_eyre::{
    Section, SectionExt,
    eyre::{Context, bail},
};
use config::Config;
use jellyfin::{ClientInfo, auth::UniqueId, connect::JsonResponseHelper};
use jellyhaj_core::{
    state::{ClientOut, LoginState, NextScreen},
    widgets::shaded::widget::{Erased, make_new_erased},
};
use jellyhaj_fetch_view::make_fetch;
use jellyhaj_form_widget::form::{FormCommandMapper, FormDataExt};
use jellyhaj_keybinds_widget::KeybindWidget;
use jellyhaj_login_widget::server::{ServerData, ServerDataAction, ServerWidget};
use jellyhaj_widgets_core::Result;
use ratatui::crossterm::style::Stylize;
use sqlx::SqliteConnection;

use crate::{DB, LoginContext};
pub fn render_select_server(cx: LoginContext, state: LoginState, out: ClientOut) -> Erased {
    let widget = ServerData::new(state.server_url.clone()).make_with_default();
    let widget = KeybindWidget::new(
        widget,
        cx.config.keybinds.form.clone(),
        FormCommandMapper::<ServerDataAction>::default(),
    );
    let widget = ServerWidget::new(widget, out, state);
    make_new_erased(cx, widget)
}

async fn get_unique(db: &mut SqliteConnection) -> Result<UniqueId> {
    let val = sqlx::query_scalar!("select id from unique_id")
        .fetch_optional(&mut *db)
        .await
        .context("retrieving unique device id")?;
    if let Some(v) = val.and_then(|v| <[u8; 64]>::try_from(v).ok().map(UniqueId)) {
        Ok(v)
    } else {
        let id = UniqueId::generate_new()?;
        let id_val = id.0.as_slice();
        sqlx::query!("insert into unique_id (id) values (?)", id_val)
            .execute(db)
            .await
            .context("storing unique device id")?;
        Ok(id)
    }
}

struct StoredCreds {
    access_token: String,
}

async fn get_stored_creds(
    db: &mut SqliteConnection,
    server_id: &str,
    store: bool,
) -> Result<Option<StoredCreds>> {
    if store {
        sqlx::query_as!(
            StoredCreds,
            "select access_token from creds where server_id = ?",
            server_id
        )
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

pub async fn connect_server(
    state: LoginState,
    out: ClientOut,
    db: DB,
    config: Arc<Config>,
    name: &'static str,
    version: &'static str,
) -> Result<NextScreen> {
    let device_name = whoami::hostname()
        .ok()
        .unwrap_or_else(|| "unknown".to_owned());
    let mut db = db.lock().await;
    let unique_id = get_unique(&mut db).await?;
    let mut client = jellyfin::JellyfinClient::new(
        state.server_url.clone(),
        ClientInfo {
            name: name.into(),
            version: version.into(),
        },
        device_name,
        unique_id,
        config.concurrent_jellyfin_connections.into(),
    )
    .context("creating jellyfin client")?;
    let status = client
        .get_system_info_public()
        .deserialize()
        .await
        .context("getting jellyfin server info")
        .with_section(|| {
            "Check the Jellyfin Server url and your internet connection".header("Hint".blue())
        })?;
    if !status.startup_wizard_completed {
        bail!(
            "Initial startup configuration is not yet supported! Please use Jellyfin Web for this step"
        );
    }
    if let Some(creds) = get_stored_creds(&mut db, &status.id, config.store_access_token)
        .await
        .context("retrieving stored authentication key")?
    {
        match client.auth_key(creds.access_token).get_self().await {
            Ok(client) => {
                return Ok(NextScreen::AuthFinished {
                    state,
                    out,
                    client,
                    server_id: status.id,
                });
            }
            Err((c, e)) => {
                tracing::warn!("error connecting with stored credentials:\n{e:?}");
                client = c.without_auth();
            }
        }
    }
    let quick_connect_available = client
        .quick_connect_enabled()
        .deserialize()
        .await
        .context("getting quick connect support status")?;
    Ok(NextScreen::SelectAuthMethod {
        state,
        out,
        client,
        quick_connect_available,
        server_id: status.id,
    })
}

pub fn render_connect_server(
    cx: LoginContext,
    state: LoginState,
    out: ClientOut,
    name: &'static str,
    version: &'static str,
) -> Erased {
    let fut = connect_server(
        state,
        out,
        cx.cache.clone(),
        cx.config.clone(),
        name,
        version,
    );
    make_fetch(cx, "Connecting to server", fut)
}
