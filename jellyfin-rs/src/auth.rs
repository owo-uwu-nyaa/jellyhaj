use std::sync::Arc;

use aws_lc_rs::digest;
use color_eyre::Result as EyreResult;
use color_eyre::eyre::eyre;
use futures_util::future::try_join;
use http::{HeaderValue, header::AUTHORIZATION};
use serde::Serialize;

use base64::{Engine, engine::general_purpose::URL_SAFE};
use tracing::{instrument, trace};

use crate::{
    Auth, AuthStatus, Authed, ClientInfo, ClientInnerAuth, JellyfinClient, KeyAuth, NoAuth,
    client_with_auth,
    connect::JsonResponseHelper,
    request::{NoQuery, RequestBuilderExt},
    session::{SessionInfo, SessionsQuery},
    user::{User, UserAuth},
};

use std::result::Result as StdResult;

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct AuthUserNameReq<'a> {
    username: &'a str,
    pw: &'a str,
}
impl JellyfinClient<NoAuth> {
    #[must_use]
    pub fn auth_key(self, key: String) -> JellyfinClient<KeyAuth> {
        let device_id = make_client_id(
            &self.inner.client.unique,
            &self.inner.client.client_info,
            &self.inner.client.device_name,
        );
        let auth_header = make_auth_header(
            &key,
            &self.inner.client.client_info,
            &self.inner.client.device_name,
            &device_id,
        );
        client_with_auth(
            self,
            KeyAuth {
                access_key: key,
                header: auth_header,
                device_id,
            },
        )
    }

    #[instrument(skip_all)]
    #[allow(clippy::future_not_send)]
    pub async fn auth_user_name(
        self,
        username: impl AsRef<str>,
        password: impl AsRef<str>,
    ) -> StdResult<JellyfinClient<Auth>, (Self, color_eyre::Report)> {
        let username = username.as_ref();
        let device_id = make_client_id(
            &self.inner.client.unique,
            &self.inner.client.client_info,
            &self.inner.client.device_name,
        );
        let auth: StdResult<UserAuth, color_eyre::Report> = async {
            self.send_request_json(
                self.post("/Users/AuthenticateByName", NoQuery)?
                    .header(
                        AUTHORIZATION,
                        make_auth_handshake_header(
                            &self.inner.client.client_info,
                            &self.inner.client.device_name,
                            &device_id,
                        ),
                    )
                    .json_body(&AuthUserNameReq {
                        username,
                        pw: password.as_ref(),
                    })?,
            )
            .deserialize()
            .await
        }
        .await;
        let auth = match auth {
            Ok(v) => v,
            Err(e) => return Err((self, e)),
        };
        let auth_header = make_auth_header(
            &auth.access_token,
            &self.inner.client.client_info,
            &self.inner.client.device_name,
            &device_id,
        );

        let auth = Auth {
            user: auth.user,
            access_token: auth.access_token,
            header: auth_header,
            device_id,
            session_id: auth.session_info.id,
        };
        Ok(make_auth_or_return(self, auth))
    }
}

impl<A: Authed> JellyfinClient<A> {
    pub async fn delete_api_key(&self, key: &str) -> EyreResult<()> {
        self.send_request(
            self.delete(
                |base: &mut String| {
                    base.push_str("/Auth/Keys/");
                    base.push_str(key);
                },
                NoQuery,
            )?
            .empty_body()?,
        )
        .await?;
        Ok(())
    }
    pub fn delete_current_api_key(&self) -> impl Future<Output = EyreResult<()>> {
        self.delete_api_key(self.get_auth().token())
    }
}

pub(crate) fn make_auth_or_return<Auth1: AuthStatus, Auth2: AuthStatus>(
    this: JellyfinClient<Auth1>,
    auth: Auth2,
) -> JellyfinClient<Auth2> {
    let inner = match Arc::try_unwrap(this.inner) {
        Ok(client) => ClientInnerAuth {
            auth,
            client: client.client,
        },
        Err(client) => ClientInnerAuth {
            auth,
            client: client.client.clone(),
        },
    };
    JellyfinClient {
        inner: Arc::new(inner),
    }
}

impl JellyfinClient<KeyAuth> {
    pub async fn get_self(self) -> StdResult<JellyfinClient<Auth>, (Self, color_eyre::Report)> {
        let user = async {
            self.send_request_json::<User>(self.get("/Users/Me", NoQuery)?.empty_body()?)
                .deserialize()
                .await
        };
        let sessions = async {
            self.get_sessions(&SessionsQuery {
                device_id: self.get_auth().device_id().into(),
                ..Default::default()
            })
            .deserialize()
            .await
        };
        let (user, mut sessions): (User, Vec<SessionInfo>) = match try_join(user, sessions).await {
            Ok(v) => v,
            Err(e) => return Err((self, e)),
        };
        if sessions.len() > 1 {
            return Err((self, eyre!("The current device has more than 1 session")));
        }
        let Some(session) = sessions.pop() else {
            return Err((self, eyre!("The current device has no associated sessions")));
        };
        let auth = Auth {
            user,
            access_token: self.inner.auth.access_key.clone(),
            header: self.inner.auth.header.clone(),
            device_id: self.inner.auth.device_id.clone(),
            session_id: session.id,
        };
        Ok(make_auth_or_return(self, auth))
    }
}

#[instrument(skip_all)]
pub(crate) fn make_auth_handshake_header(
    client_info: &ClientInfo,
    device_name: &str,
    device_id: &str,
) -> HeaderValue {
    let mut val = r#"MediaBrowser Client=""#.to_string();
    val += &client_info.name;
    val += r#"", Version=""#;
    val += &client_info.version;
    val += r#"", Device=""#;
    URL_SAFE.encode_string(device_name.as_bytes(), &mut val);
    val += r#"", DeviceId=""#;
    val += device_id;
    val.push('"');
    trace!("header value: {val}");
    HeaderValue::try_from(val).expect("invalid client info for header value")
}

#[instrument(skip_all)]
pub(crate) fn make_auth_header(
    access_token: &str,
    client_info: &ClientInfo,
    device_name: &str,
    device_id: &str,
) -> HeaderValue {
    let mut val = r#"MediaBrowser Token=""#.to_string();
    val += access_token;
    val += r#"", Client=""#;
    val += &client_info.name;
    val += r#"", Version=""#;
    val += &client_info.version;
    val += r#"", Device=""#;
    URL_SAFE.encode_string(device_name.as_bytes(), &mut val);
    val += r#"", DeviceId=""#;
    val += device_id;
    val.push('"');
    HeaderValue::try_from(val).expect("invalid client info for header value")
}

#[derive(Debug, Clone, Copy)]
pub struct UniqueId(pub [u8; 64]);

impl UniqueId {
    pub fn generate_new() -> Result<Self, getrandom::Error> {
        let mut this = [0u8; 64];
        getrandom::fill(&mut this)?;
        Ok(Self(this))
    }
}

#[instrument(skip_all)]
pub(crate) fn make_client_id(
    unique: &UniqueId,
    client_info: &ClientInfo,
    device_name: &str,
) -> String {
    let mut digest = digest::Context::new(&digest::SHA256);
    digest.update(client_info.name.as_bytes());
    digest.update(client_info.version.as_bytes());
    digest.update(device_name.as_bytes());
    digest.update(&unique.0);
    let hash = digest.finish();
    URL_SAFE.encode(hash)
}
