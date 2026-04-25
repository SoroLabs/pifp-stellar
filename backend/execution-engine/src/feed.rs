use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::{connect_async, tungstenite::Message};
use tracing::{debug, info, warn};

use crate::types::PoolSnapshot;

#[derive(Debug, thiserror::Error)]
pub enum FeedError {
    #[error("websocket error: {0}")]
    WebSocket(String),
    #[error("json error: {0}")]
    Json(String),
}

pub type Result<T> = std::result::Result<T, FeedError>;

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum SnapshotMessage {
    Single(PoolSnapshot),
    Batch(Vec<PoolSnapshot>),
}

pub async fn stream_pool_snapshots(ws_url: &str, sender: mpsc::Sender<PoolSnapshot>) -> Result<()> {
    info!("connecting to websocket feed: {ws_url}");
    let (stream, _) = connect_async(ws_url)
        .await
        .map_err(|e| FeedError::WebSocket(e.to_string()))?;
    let (_, mut read) = stream.split();

    while let Some(message) = read.next().await {
        let message = message.map_err(|e| FeedError::WebSocket(e.to_string()))?;
        let text = match message {
            Message::Text(text) => text,
            Message::Binary(bytes) => match String::from_utf8(bytes.to_vec()) {
                Ok(text) => text,
                Err(err) => {
                    warn!("dropping non-utf8 websocket frame: {err}");
                    continue;
                }
            },
            Message::Ping(_) | Message::Pong(_) => continue,
            Message::Close(_) => break,
            _ => continue,
        };

        debug!("received websocket message: {text}");
        let payload: SnapshotMessage = serde_json::from_str(&text)
            .map_err(|e| FeedError::Json(format!("invalid snapshot payload: {e}")))?;

        match payload {
            SnapshotMessage::Single(snapshot) => {
                sender
                    .send(snapshot)
                    .await
                    .map_err(|e| FeedError::WebSocket(e.to_string()))?;
            }
            SnapshotMessage::Batch(snapshots) => {
                for snapshot in snapshots {
                    sender
                        .send(snapshot)
                        .await
                        .map_err(|e| FeedError::WebSocket(e.to_string()))?;
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_single_snapshot() {
        let json = r#"{
            "pool_id":"pool-1",
            "base_asset":"A",
            "quote_asset":"B",
            "base_reserve":1000.0,
            "quote_reserve":1010.0,
            "fee_bps":30,
            "updated_ledger":7
        }"#;

        let payload: SnapshotMessage = serde_json::from_str(json).unwrap();
        match payload {
            SnapshotMessage::Single(snapshot) => {
                assert_eq!(snapshot.pool_id, "pool-1");
                assert_eq!(snapshot.fee_bps, 30);
            }
            SnapshotMessage::Batch(_) => panic!("expected single snapshot"),
        }
    }
}
