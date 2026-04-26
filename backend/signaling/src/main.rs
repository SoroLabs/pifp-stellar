use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
    routing::get,
    Router,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{info, warn};

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "camelCase")]
enum SignalingMessage {
    Join {
        room_id: String,
    },
    Signal {
        room_id: String,
        data: serde_json::Value,
    },
}

struct AppState {
    // Room ID -> Vec of (Peer ID, Sender)
    rooms: DashMap<String, Vec<(String, mpsc::UnboundedSender<Message>)>>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = Arc::new(AppState {
        rooms: DashMap::new(),
    });

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await.unwrap();
    info!("Signaling server listening on 0.0.0.0:3001");
    axum::serve(listener, app).await.unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    axum::extract::State(state): axum::extract::State<Arc<AppState>>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: Arc<AppState>) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel();

    // Spawn a task to handle outgoing messages
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let peer_id = uuid::Uuid::new_v4().to_string();
    let mut current_room: Option<String> = None;

    while let Some(Ok(msg)) = receiver.next().await {
        if let Message::Text(text) = msg {
            if let Ok(signaling_msg) = serde_json::from_str::<SignalingMessage>(&text) {
                match signaling_msg {
                    SignalingMessage::Join { room_id } => {
                        info!("Peer {} joining room {}", peer_id, room_id);
                        current_room = Some(room_id.clone());
                        state
                            .rooms
                            .entry(room_id)
                            .or_default()
                            .push((peer_id.clone(), tx.clone()));
                    }
                    SignalingMessage::Signal { room_id, data } => {
                        if let Some(peers) = state.rooms.get(&room_id) {
                            for (id, peer_tx) in peers.iter() {
                                if id != &peer_id {
                                    let relay_msg = SignalingMessage::Signal {
                                        room_id: room_id.clone(),
                                        data: data.clone(),
                                    };
                                    if let Ok(relay_text) = serde_json::to_string(&relay_msg) {
                                        let _ = peer_tx.send(Message::Text(relay_text.into()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    // Cleanup
    if let Some(room_id) = current_room {
        if let Some(mut peers) = state.rooms.get_mut(&room_id) {
            peers.retain(|(id, _)| id != &peer_id);
            if peers.is_empty() {
                drop(peers);
                state.rooms.remove(&room_id);
            }
        }
    }
    send_task.abort();
}
