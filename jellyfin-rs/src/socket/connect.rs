use super::TraceResult;
use std::{cmp::min, time::Duration};
use tokio::time::sleep;
use tokio_websockets::WebSocketStream;
use tracing::{info, instrument};

use crate::{connect::MaybeTls, socket::ConnectInfo};
use color_eyre::Result;

async fn make_socket_inner(config: &ConnectInfo) -> Result<WebSocketStream<MaybeTls>> {
    let (stream, _) = tokio_websockets::client::Builder::from_uri(config.uri.clone())
        .connect_on(config.config.http1_base_connection().await?)
        .await?;
    Ok(stream)
}

#[instrument(skip_all)]
pub async fn make_socket(config: &ConnectInfo) -> WebSocketStream<MaybeTls> {
    if let Some(socket) = make_socket_inner(config).await.trace_err() {
        return socket;
    }
    let mut backoff = Duration::from_secs(1);
    loop {
        info!("reconnecting in {} seconds", backoff.as_secs());
        sleep(backoff).await;
        if let Some(socket) = make_socket_inner(config).await.trace_err() {
            return socket;
        }
        backoff = min(backoff * 2, Duration::from_mins(1));
    }
}
