use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Deserialize)]
pub struct SubmitIntentRequest {
    pub domain: String,
    pub chain_id: String,
    pub donor: String,
    pub project_id: u64,
    pub amount_stroops: u64,
    pub nonce: u64,
    pub expires_at: u64,
    pub message: String,
    pub signature: String,
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SubmitIntentResponse {
    pub accepted: bool,
    pub intent_id: String,
    pub pending_balance_stroops: u64,
    pub queue_depth: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollupBalance {
    pub address: String,
    pub pending_stroops: u64,
    pub confirmed_stroops: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SettledIntent {
    pub intent_id: String,
    pub donor: String,
    pub project_id: u64,
    pub amount_stroops: u64,
    pub nonce: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RollupBatch {
    pub batch_id: u64,
    pub settled_at_unix: u64,
    pub intent_count: usize,
    pub total_amount_stroops: u64,
    pub state_root: String,
    pub soroban_batch_tx: String,
    pub intents: Vec<SettledIntent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TriggerSettlementResponse {
    pub settled: bool,
    pub batch: Option<RollupBatch>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

#[derive(Debug, Clone)]
struct StoredIntent {
    intent_id: String,
    donor: String,
    project_id: u64,
    amount_stroops: u64,
    nonce: u64,
}

#[derive(Default)]
struct RollupLedger {
    queue: Vec<StoredIntent>,
    pending_by_donor: HashMap<String, u64>,
    confirmed_by_donor: HashMap<String, u64>,
    last_nonce_by_donor: HashMap<String, u64>,
    batches: Vec<RollupBatch>,
    next_batch_id: u64,
}

pub struct RollupState {
    ledger: Mutex<RollupLedger>,
    pub settle_interval: Duration,
}

pub fn router(state: Arc<RollupState>) -> Router {
    Router::new()
        .route("/rollup/intents", post(submit_intent))
    .route("/rollup/balance/{address}", get(get_balance))
        .route("/rollup/batches", get(get_batches))
        .route("/rollup/settle", post(settle_now))
        .with_state(state)
}

async fn submit_intent(
    State(state): State<Arc<RollupState>>,
    Json(req): Json<SubmitIntentRequest>,
) -> Result<Json<SubmitIntentResponse>, (StatusCode, Json<ErrorResponse>)> {
    match state.submit(req) {
        Ok(response) => Ok(Json(response)),
        Err(message) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse { error: message }),
        )),
    }
}

async fn get_balance(
    State(state): State<Arc<RollupState>>,
    Path(address): Path<String>,
) -> Json<RollupBalance> {
    Json(state.balance_for(&address))
}

async fn get_batches(State(state): State<Arc<RollupState>>) -> Json<Vec<RollupBatch>> {
    Json(state.batches())
}

async fn settle_now(State(state): State<Arc<RollupState>>) -> Json<TriggerSettlementResponse> {
    let batch = state.settle_pending();
    Json(TriggerSettlementResponse {
        settled: batch.is_some(),
        batch,
    })
}

impl RollupState {
    pub fn new(settle_interval: Duration) -> Self {
        Self {
            ledger: Mutex::new(RollupLedger {
                next_batch_id: 1,
                ..RollupLedger::default()
            }),
            settle_interval,
        }
    }

    pub fn start_settlement_loop(self: Arc<Self>) {
        let interval_duration = self.settle_interval;
        tokio::spawn(async move {
            let mut ticker = tokio::time::interval(interval_duration);
            loop {
                ticker.tick().await;
                let _ = self.settle_pending();
            }
        });
    }

    fn submit(&self, req: SubmitIntentRequest) -> Result<SubmitIntentResponse, String> {
        validate_intent(&req)?;

        let canonical = canonical_intent_message(
            &req.domain,
            &req.chain_id,
            &req.donor,
            req.project_id,
            req.amount_stroops,
            req.nonce,
            req.expires_at,
        );

        if req.message != canonical {
            return Err("Typed message does not match payload".to_string());
        }

        verify_signature(&req.public_key, &req.signature, canonical.as_bytes())?;
        let intent_id = compute_intent_id(&req.message, &req.signature);

        let mut guard = self.ledger.lock().unwrap();
        let last_nonce = guard
            .last_nonce_by_donor
            .get(&req.donor)
            .copied()
            .unwrap_or_default();

        if req.nonce <= last_nonce {
            return Err("Nonce must strictly increase per donor".to_string());
        }

        guard
            .last_nonce_by_donor
            .insert(req.donor.clone(), req.nonce);

        let pending_balance_stroops = {
            let pending_balance = guard.pending_by_donor.entry(req.donor.clone()).or_insert(0);
            *pending_balance = pending_balance.saturating_add(req.amount_stroops);
            *pending_balance
        };

        guard.queue.push(StoredIntent {
            intent_id: intent_id.clone(),
            donor: req.donor.clone(),
            project_id: req.project_id,
            amount_stroops: req.amount_stroops,
            nonce: req.nonce,
        });

        Ok(SubmitIntentResponse {
            accepted: true,
            intent_id,
            pending_balance_stroops,
            queue_depth: guard.queue.len(),
        })
    }

    fn settle_pending(&self) -> Option<RollupBatch> {
        let mut guard = self.ledger.lock().unwrap();
        if guard.queue.is_empty() {
            return None;
        }

        let batch_id = guard.next_batch_id;
        guard.next_batch_id = guard.next_batch_id.saturating_add(1);

        let settled_at_unix = now_unix();
        let mut total_amount_stroops = 0u64;
        let queue = std::mem::take(&mut guard.queue);

        let mut settled_intents = Vec::with_capacity(queue.len());
        let mut root_hasher = Sha256::new();

        for intent in queue {
            total_amount_stroops = total_amount_stroops.saturating_add(intent.amount_stroops);
            root_hasher.update(intent.intent_id.as_bytes());

            let pending = guard
                .pending_by_donor
                .entry(intent.donor.clone())
                .or_insert(0);
            *pending = pending.saturating_sub(intent.amount_stroops);

            let confirmed = guard
                .confirmed_by_donor
                .entry(intent.donor.clone())
                .or_insert(0);
            *confirmed = confirmed.saturating_add(intent.amount_stroops);

            settled_intents.push(SettledIntent {
                intent_id: intent.intent_id,
                donor: intent.donor,
                project_id: intent.project_id,
                amount_stroops: intent.amount_stroops,
                nonce: intent.nonce,
            });
        }

        let state_root = hex::encode(root_hasher.finalize());
        let soroban_batch_tx = format!(
            "soroban://batch_settle?batch_id={batch_id}&intents={}&state_root={state_root}",
            settled_intents.len()
        );

        let batch = RollupBatch {
            batch_id,
            settled_at_unix,
            intent_count: settled_intents.len(),
            total_amount_stroops,
            state_root,
            soroban_batch_tx,
            intents: settled_intents,
        };

        guard.batches.push(batch.clone());
        Some(batch)
    }

    fn balance_for(&self, address: &str) -> RollupBalance {
        let guard = self.ledger.lock().unwrap();
        RollupBalance {
            address: address.to_string(),
            pending_stroops: guard.pending_by_donor.get(address).copied().unwrap_or(0),
            confirmed_stroops: guard.confirmed_by_donor.get(address).copied().unwrap_or(0),
        }
    }

    fn batches(&self) -> Vec<RollupBatch> {
        let guard = self.ledger.lock().unwrap();
        guard.batches.clone()
    }
}

fn validate_intent(req: &SubmitIntentRequest) -> Result<(), String> {
    if req.domain.trim().is_empty() {
        return Err("domain is required".to_string());
    }
    if req.chain_id.trim().is_empty() {
        return Err("chain_id is required".to_string());
    }
    if req.donor.trim().is_empty() {
        return Err("donor is required".to_string());
    }
    if req.amount_stroops == 0 {
        return Err("amount_stroops must be greater than zero".to_string());
    }

    if req.expires_at <= now_unix() {
        return Err("intent has expired".to_string());
    }

    Ok(())
}

pub fn canonical_intent_message(
    domain: &str,
    chain_id: &str,
    donor: &str,
    project_id: u64,
    amount_stroops: u64,
    nonce: u64,
    expires_at: u64,
) -> String {
    format!(
        "PIFP_ROLLUP_INTENT\n\\
        domain:{domain}\n\\
        chain_id:{chain_id}\n\\
        donor:{donor}\n\\
        project_id:{project_id}\n\\
        amount_stroops:{amount_stroops}\n\\
        nonce:{nonce}\n\\
        expires_at:{expires_at}"
    )
}

fn verify_signature(public_key_b64: &str, signature_b64: &str, message: &[u8]) -> Result<(), String> {
    let public_key_bytes = BASE64
        .decode(public_key_b64)
        .map_err(|_| "public_key is not valid base64".to_string())?;
    let signature_bytes = BASE64
        .decode(signature_b64)
        .map_err(|_| "signature is not valid base64".to_string())?;

    let public_key_arr: [u8; 32] = public_key_bytes
        .try_into()
        .map_err(|_| "public_key must decode to 32 bytes".to_string())?;
    let signature_arr: [u8; 64] = signature_bytes
        .try_into()
        .map_err(|_| "signature must decode to 64 bytes".to_string())?;

    let key = VerifyingKey::from_bytes(&public_key_arr)
        .map_err(|_| "invalid ed25519 public key".to_string())?;
    let signature = Signature::from_bytes(&signature_arr);

    key.verify(message, &signature)
        .map_err(|_| "signature verification failed".to_string())
}

fn compute_intent_id(message: &str, signature_b64: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(message.as_bytes());
    hasher.update(signature_b64.as_bytes());
    hex::encode(hasher.finalize())
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    fn build_signed_request(amount_stroops: u64, nonce: u64) -> SubmitIntentRequest {
        let signing_key = SigningKey::from_bytes(&[7u8; 32]);
        let public_key = signing_key.verifying_key();

        let expires_at = now_unix() + 120;
        let message = canonical_intent_message(
            "pifp-rollup-v1",
            "soroban-testnet",
            "GABC123DONOR",
            42,
            amount_stroops,
            nonce,
            expires_at,
        );

        let signature = signing_key.sign(message.as_bytes());

        SubmitIntentRequest {
            domain: "pifp-rollup-v1".to_string(),
            chain_id: "soroban-testnet".to_string(),
            donor: "GABC123DONOR".to_string(),
            project_id: 42,
            amount_stroops,
            nonce,
            expires_at,
            message,
            signature: BASE64.encode(signature.to_bytes()),
            public_key: BASE64.encode(public_key.to_bytes()),
        }
    }

    #[test]
    fn accepts_valid_signed_intent_and_tracks_pending_balance() {
        let state = RollupState::new(Duration::from_secs(30));
        let req = build_signed_request(10_000, 1);

        let response = state.submit(req).expect("request should succeed");
        assert!(response.accepted);
        assert_eq!(response.pending_balance_stroops, 10_000);
        assert_eq!(response.queue_depth, 1);

        let balance = state.balance_for("GABC123DONOR");
        assert_eq!(balance.pending_stroops, 10_000);
        assert_eq!(balance.confirmed_stroops, 0);
    }

    #[test]
    fn rejects_invalid_signature() {
        let state = RollupState::new(Duration::from_secs(30));
        let mut req = build_signed_request(10_000, 1);
        req.signature = BASE64.encode([0u8; 64]);

        let error = state.submit(req).expect_err("signature should fail");
        assert!(error.contains("verification"));
    }

    #[test]
    fn settlement_moves_pending_to_confirmed() {
        let state = RollupState::new(Duration::from_secs(30));
        state
            .submit(build_signed_request(5_000, 1))
            .expect("first intent should succeed");
        state
            .submit(build_signed_request(7_000, 2))
            .expect("second intent should succeed");

        let batch = state
            .settle_pending()
            .expect("batch should be created with pending intents");
        assert_eq!(batch.intent_count, 2);
        assert_eq!(batch.total_amount_stroops, 12_000);

        let balance = state.balance_for("GABC123DONOR");
        assert_eq!(balance.pending_stroops, 0);
        assert_eq!(balance.confirmed_stroops, 12_000);
    }
}
