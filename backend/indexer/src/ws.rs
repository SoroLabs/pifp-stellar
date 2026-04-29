//! WebSocket server for real-time event broadcasting.
//!
//! ## Protocol
//!
//! After connecting, clients may send a JSON subscription filter:
//! ```json
//! { "type": "subscribe", "event_types": ["project_funded", "funds_released"], "project_ids": ["42"] }
//! ```
//! Omitting a field means "accept all". An empty `event_types` array also means "accept all".
//!
//! The server sends:
//! - `{ "type": "event", "payload": <PifpEvent> }` — a new contract event
//! - `{ "type": "pong" }` — response to a `{ "type": "ping" }` from the client
//! - `{ "type": "connected", "message": "..." }` — sent immediately on connect
//!
//! ## Resilience
//!
//! * A `broadcast::channel` with capacity 512 acts as an in-memory event queue,
//!   absorbing bursts without dropping events for slow consumers (up to the buffer).
//! * Each connection task has its own `broadcast::Receiver`; lagged receivers
//!   receive a `Lagged` error and are disconnected gracefully.
//! * A 30-second ping/pong heartbeat detects dead connections.

use std::collections::HashSet;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use tokio::net::TcpListener;
use tokio::sync::broadcast;
use tokio::time::interval;
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use tracing::{debug, error, info, warn};

use crate::events::PifpEvent;

// ─── Channel capacity ────────────────────────────────────────────────────────
/// In-memory event queue depth. Absorbs bursts; slow clients are disconnected
/// when they fall more than this many messages behind.
const BROADCAST_CAPACITY: usize = 512;

/// How often the server sends a ping frame to each client.
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(30);

// ─── Wire protocol types ─────────────────────────────────────────────────────

/// Outbound message envelope sent to clients.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage<'a> {
    Connected { message: &'static str },
    Event { payload: &'a PifpEvent },
    Pong,
}

/// Inbound message envelope received from clients.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ClientMessage {
    /// Subscribe / update subscription filter.
    Subscribe {
        /// Event type strings to accept, e.g. `["project_funded"]`.
        /// Empty or absent → accept all event types.
        #[serde(default)]
        event_types: Vec<String>,
        /// Project IDs to accept, e.g. `["42", "7"]`.
        /// Empty or absent → accept all projects.
        #[serde(default)]
        project_ids: Vec<String>,
    },
    Ping,
}

/// Per-client subscription filter, updated on each `subscribe` message.
#[derive(Debug, Default, Clone)]
struct Filter {
    /// `None` means accept all event types.
    event_types: Option<HashSet<String>>,
    /// `None` means accept all project IDs.
    project_ids: Option<HashSet<String>>,
}

impl Filter {
    fn from_subscribe(event_types: Vec<String>, project_ids: Vec<String>) -> Self {
        Self {
            event_types: if event_types.is_empty() {
                None
            } else {
                Some(event_types.into_iter().collect())
            },
            project_ids: if project_ids.is_empty() {
                None
            } else {
                Some(project_ids.into_iter().collect())
            },
        }
    }

    fn matches(&self, event: &PifpEvent) -> bool {
        if let Some(types) = &self.event_types {
            if !types.contains(&event.event_type) {
                return false;
            }
        }
        if let Some(ids) = &self.project_ids {
            match &event.project_id {
                Some(pid) if ids.contains(pid) => {}
                _ => return false,
            }
        }
        true
    }
}

// ─── Shared state ─────────────────────────────────────────────────────────────

/// Shared handle passed to the indexer so it can enqueue events.
#[derive(Clone)]
pub struct WsState {
    /// Broadcast sender — the in-memory event queue.
    pub tx: broadcast::Sender<Arc<PifpEvent>>,
}

impl WsState {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        Self { tx }
    }

    /// Enqueue an event for all connected clients.
    /// Silently drops if there are no subscribers (expected at startup).
    pub fn broadcast_event(&self, event: &PifpEvent) {
        let _ = self.tx.send(Arc::new(event.clone()));
    }
}

// ─── Server ───────────────────────────────────────────────────────────────────

pub async fn run(addr: String, state: WsState) {
    let listener = TcpListener::bind(&addr)
        .await
        .expect("Failed to bind WebSocket address");
    info!("WebSocket server listening on ws://{}", addr);

    while let Ok((stream, peer_addr)) = listener.accept().await {
        let ws_stream = match tokio_tungstenite::accept_async(stream).await {
            Ok(ws) => ws,
            Err(e) => {
                warn!("WebSocket handshake failed from {peer_addr}: {e}");
                continue;
            }
        };

        let rx = state.tx.subscribe();
        tokio::spawn(handle_connection(ws_stream, peer_addr, rx));
    }
}

// ─── Per-connection handler ───────────────────────────────────────────────────

async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    peer: SocketAddr,
    mut rx: broadcast::Receiver<Arc<PifpEvent>>,
) {
    info!("WebSocket client connected: {peer}");

    let (mut write, mut read) = ws_stream.split();
    let mut filter = Filter::default();
    let mut heartbeat = interval(HEARTBEAT_INTERVAL);
    // Skip the first tick which fires immediately.
    heartbeat.tick().await;

    // Send a welcome message.
    let welcome = serde_json::to_string(&ServerMessage::Connected {
        message: "Connected to PIFP event stream",
    })
    .unwrap_or_default();
    if write
        .send(Message::Text(Utf8Bytes::from(welcome)))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            // ── Inbound client message ──────────────────────────────────────
            msg = read.next() => {
                match msg {
                    None | Some(Ok(Message::Close(_))) => {
                        debug!("WebSocket client disconnected: {peer}");
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WebSocket error from {peer}: {e}");
                        break;
                    }
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<ClientMessage>(&text) {
                            Ok(ClientMessage::Subscribe { event_types, project_ids }) => {
                                filter = Filter::from_subscribe(event_types, project_ids);
                                debug!("Updated filter for {peer}: {:?}", filter);
                            }
                            Ok(ClientMessage::Ping) => {
                                let pong = serde_json::to_string(&ServerMessage::Pong)
                                    .unwrap_or_default();
                                if write
                                    .send(Message::Text(Utf8Bytes::from(pong)))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => {
                                debug!("Unrecognised message from {peer}: {e}");
                            }
                        }
                    }
                    Some(Ok(Message::Ping(data))) => {
                        // Respond to protocol-level pings.
                        if write.send(Message::Pong(data)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {} // binary / pong frames ignored
                }
            }

            // ── Outbound broadcast event ────────────────────────────────────
            result = rx.recv() => {
                match result {
                    Ok(event) => {
                        if !filter.matches(&event) {
                            continue;
                        }
                        match serde_json::to_string(&ServerMessage::Event { payload: &event }) {
                            Ok(json) => {
                                if write
                                    .send(Message::Text(Utf8Bytes::from(json)))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                            Err(e) => error!("Failed to serialise event: {e}"),
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        warn!("Client {peer} lagged by {n} messages — disconnecting");
                        break;
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }

            // ── Heartbeat ───────────────────────────────────────────────────
            _ = heartbeat.tick() => {
                if write.send(Message::Ping(vec![].into())).await.is_err() {
                    break;
                }
            }
        }
    }

    info!("WebSocket connection closed: {peer}");
}
