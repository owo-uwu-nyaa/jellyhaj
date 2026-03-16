mod connect;
mod poll_socket;

use connect::make_socket;
use futures_util::StreamExt;
use poll_socket::PollSocket;

use futures_core::Stream;
use http::Uri;
use serde::{Deserialize, Serialize};
use tracing::error;

use crate::{connect::ConnectionConfig, items::UserData, Auth, JellyfinClient, Result};

trait TraceResult<T> {
    fn trace_err(self) -> Option<T>;
}

impl<T> TraceResult<T> for Result<T> {
    fn trace_err(self) -> Option<T> {
        match self {
            Ok(v) => Some(v),
            Err(e) => {
                error!("encountered error: {e:?}");
                None
            }
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "MessageType")]
pub enum JellyfinMessage {
    #[serde(rename_all = "PascalCase")]
    RefreshProgress { data: RefreshProgress },
    #[serde(rename_all = "PascalCase")]
    UserDataChanged { data: UserDataChanged },
    #[serde(rename_all = "PascalCase")]
    LibraryChanged { data: LibraryChanged },
    #[serde(untagged)]
    #[serde(rename_all = "PascalCase")]
    Unknown {
        message_type: String,
        data: serde_json::Value,
    },
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct RefreshProgress {
    pub item_id: String,
    pub progress: f32,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct UserDataChanged {
    pub user_data_list: Vec<ChangedUserData>,
    pub user_id: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct ChangedUserData {
    pub item_id: String,
    pub key: String,
    #[serde(flatten)]
    pub user_data: UserData,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct LibraryChanged {
    pub collection_folders: Vec<String>,
    pub folders_added_to: Vec<String>,
    pub folders_removed_from: Vec<String>,
    pub items_added: Vec<String>,
    pub items_removed: Vec<String>,
    pub items_updated: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "MessageType")]
enum JellyfinMessageInternal {
    KeepAlive,
    #[serde(rename_all = "PascalCase")]
    ForceKeepAlive {
        data: u64,
    },
    #[serde(untagged)]
    Public(JellyfinMessage),
}

#[cfg(test)]
mod serde_tests {
    use serde_json::json;

    use crate::socket::{JellyfinMessage, RefreshProgress};

    #[test]
    fn serialize() {
        let val = serde_json::to_value(JellyfinMessage::RefreshProgress {
            data: RefreshProgress {
                item_id: "1".to_string(),
                progress: 1.0,
            },
        })
        .expect("serializing message to value");
        let expected =
            json!({"MessageType": "RefreshProgress", "Data": {"ItemId": "1", "Progress": 1.0}});
        assert_eq!(expected, val)
    }
}

#[derive(Debug, Default, Clone, Serialize)]
struct SocketQuery<'s> {
    api_key: &'s str,
    deviceid: &'s str,
}

struct ConnectInfo {
    uri: Uri,
    config: ConnectionConfig,
}

impl JellyfinClient<Auth> {
    pub fn get_socket(&self) -> Result<impl Stream<Item = JellyfinMessage> + 'static + Send> {
        let uri = http::uri::Builder::new()
            .scheme(if self.config.tls { "wss" } else { "ws" })
            .authority(self.config.authority.clone())
            .path_and_query(self.build_uri(
                "/socket",
                SocketQuery {
                    api_key: &self.inner.auth.access_token,
                    deviceid: &self.inner.auth.device_id,
                },
            )?)
            .build()?;

        let connect = ConnectInfo {
            uri,
            config: self.config.clone(),
        };
        let connections = futures_util::stream::unfold(connect, |info| async move {
            let socket = make_socket(&info).await;
            Some((PollSocket::new(socket), info))
        });
        Ok(connections.flatten())
    }
}
