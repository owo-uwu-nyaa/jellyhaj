use serde::{Deserialize, Serialize};

use crate::{
    AuthStatus, JellyfinClient, Result,
    connect::JsonResponse,
    request::{NoQuery, RequestBuilderExt},
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
#[cfg_attr(feature = "valuable", derive(valuable::Valuable))]
pub struct PublicSysteminfo {
    pub server_name: String,
    pub version: String,
    pub product_name: String,
    pub id: String,
    pub startup_wizard_completed: bool,
}

impl<A: AuthStatus> JellyfinClient<A> {
    pub async fn get_system_info_public(&self) -> Result<JsonResponse<PublicSysteminfo>> {
        self.send_request_json(self.get("System/Info/Public", NoQuery)?.empty_body()?)
            .await
    }
}
