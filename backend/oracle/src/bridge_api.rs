use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeMessage {
    pub id: String,
    pub source_chain: String,
    pub target_chain: String,
    pub amount: String,
    pub recipient: String,
    pub status: String,
    pub signatures_collected: usize,
    pub total_required: usize,
}

pub struct BridgeState {
    pub messages: Mutex<HashMap<String, BridgeMessage>>,
}

pub fn router() -> Router<Arc<crate::health::ServerState>> {
    Router::new()
        .route("/bridge/messages", get(get_messages))
    .route("/bridge/messages/{id}", get(get_message))
    .route("/bridge/sign/{id}", post(add_signature))
        .with_state(state)
}

async fn get_messages(State(state): State<Arc<crate::health::ServerState>>) -> Json<Vec<BridgeMessage>> {
    let messages = state.bridge.messages.lock().unwrap();
    Json(messages.values().cloned().collect())
}

async fn get_message(
    State(state): State<Arc<crate::health::ServerState>>,
    Path(id): Path<String>,
) -> Json<Option<BridgeMessage>> {
    let messages = state.bridge.messages.lock().unwrap();
    Json(messages.get(&id).cloned())
}

async fn add_signature(
    State(state): State<Arc<crate::health::ServerState>>,
    Path(id): Path<String>,
) -> Json<bool> {
    let mut messages = state.bridge.messages.lock().unwrap();
    if let Some(msg) = messages.get_mut(&id) {
        if msg.signatures_collected < msg.total_required {
            msg.signatures_collected += 1;
            if msg.signatures_collected >= msg.total_required {
                msg.status = "Signed".to_string();
            }
            return Json(true);
        }
    }
    Json(false)
}

impl BridgeState {
    pub fn new() -> Self {
        let mut messages = HashMap::new();
        // Seed with some mock data for the UI
        messages.insert(
            "tx_001".to_string(),
            BridgeMessage {
                id: "tx_001".to_string(),
                source_chain: "Ethereum".to_string(),
                target_chain: "Soroban".to_string(),
                amount: "1000.00".to_string(),
                recipient: "GB...".to_string(),
                status: "Pending".to_string(),
                signatures_collected: 2,
                total_required: 5,
            },
        );
        Self {
            messages: Mutex::new(messages),
        }
    }
}
