use crate::{
    Auth, JellyfinClient, NoAuth,
    auth::{make_auth_handshake_header, make_auth_header, make_auth_or_return, make_client_id},
    connect::JsonResponse,
    request::{NoQuery, RequestBuilderExt},
    user::UserAuth,
};

use color_eyre::Result;
use http::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};
use std::result::Result as StdResult;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct QuickConnectStatus {
    pub authenticated: bool,
    pub secret: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize)]
struct QuickConnectAuthorizeQuery<'c> {
    code: &'c str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
struct QuickConnectStatusQuery<'s> {
    secret: &'s str,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "PascalCase")]
struct AuthQuickConnectReq<'s> {
    secret: &'s str,
}

impl JellyfinClient<NoAuth> {
    pub async fn quick_connect_enabled(&self) -> Result<JsonResponse<bool>> {
        self.send_request_json(self.get("/QuickConnect/Enabled", NoQuery)?.empty_body()?)
            .await
    }

    pub async fn initiate_quick_connect(&self) -> Result<JsonResponse<QuickConnectStatus>> {
        let device_id = make_client_id(
            &self.inner.unique,
            &self.inner.client_info,
            &self.inner.device_name,
        );
        self.send_request_json(
            self.post("/QuickConnect/Initiate", NoQuery)?
                .header(
                    AUTHORIZATION,
                    make_auth_handshake_header(
                        &self.inner.client_info,
                        &self.inner.device_name,
                        &device_id,
                    ),
                )
                .empty_body()?,
        )
        .await
    }

    pub async fn get_quick_connect_status(
        &self,
        secret: &str,
    ) -> Result<JsonResponse<QuickConnectStatus>> {
        self.send_request_json(
            self.get("/QuickConnect/Connect", &QuickConnectStatusQuery { secret })?
                .empty_body()?,
        )
        .await
    }

    pub async fn auth_quick_connect(
        self,
        secret: &str,
    ) -> StdResult<JellyfinClient, (Self, color_eyre::Report)> {
        let auth: Result<UserAuth> = async {
            self.send_request_json(
                self.post("/Users/AuthenticateWithQuickConnect", NoQuery)?
                    .json_body(&AuthQuickConnectReq { secret })?,
            )
            .await?
            .deserialize()
            .await
        }
        .await;
        let auth = match auth {
            Ok(v) => v,
            Err(e) => return Err((self, e)),
        };

        let device_id = make_client_id(
            &self.inner.unique,
            &self.inner.client_info,
            &self.inner.device_name,
        );
        let auth_header = make_auth_header(
            &auth.access_token,
            &self.inner.client_info,
            &self.inner.device_name,
            &device_id,
        );

        let auth = Auth {
            user: auth.user,
            access_token: auth.access_token,
            header: auth_header,
            device_id,
        };
        Ok(make_auth_or_return(self, auth))
    }
}

impl JellyfinClient {
    pub async fn authorize_quick_connect(&self, code: &str) -> Result<JsonResponse<bool>> {
        self.send_request_json(
            self.post(
                "/QuickConnect/Authorize",
                &QuickConnectAuthorizeQuery { code },
            )?
            .empty_body()?,
        )
        .await
    }
}
