use std::collections::HashMap;
use std::sync::Arc;
use axum::{
    extract::{Query, State},
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use crate::state_proof::{StateMerkleTree, MerkleProof};

#[derive(Deserialize)]
pub struct OffchainComputeParams {
    pub project_id: u64,
}

#[derive(Serialize)]
pub struct OffchainComputeResponse {
    pub project_id: u64,
    pub result: String,
    pub state_root: String,
    pub proofs: Vec<MerkleProof>,
    pub ledger_seq: u32,
}

pub fn router() -> Router<()> {
    Router::new().route("/compute", get(handle_offchain_compute))
}

async fn handle_offchain_compute(
    Query(params): Query<OffchainComputeParams>,
) -> impl IntoResponse {
    // 1. Simulate fetching contract state from a "local snapshot"
    // In a real scenario, this would be fetched from Soroban RPC or a local DB
    let mut state = HashMap::new();
    state.insert("project_id".to_string(), params.project_id.to_string());
    state.insert("total_donations".to_string(), "50000".to_string());
    state.insert("donor_count".to_string(), "120".to_string());
    state.insert("avg_donation".to_string(), "416".to_string());

    // 2. Build the Merkle Tree for the state
    let tree = StateMerkleTree::new(&state);
    let state_root = tree.root();

    // 3. Perform the "Off-chain Computation"
    // Example: Calculate the impact score or something heavy
    let total: u64 = state.get("total_donations").unwrap().parse().unwrap_or(0);
    let count: u64 = state.get("donor_count").unwrap().parse().unwrap_or(1);
    let result = format!("Impact Score: {:.2}", (total as f64 * 0.8) + (count as f64 * 0.2));

    // 4. Generate proofs for all state variables used
    let mut proofs = Vec::new();
    for i in 0..state.len() {
        if let Some(proof) = tree.get_proof(i) {
            proofs.push(proof);
        }
    }

    Json(OffchainComputeResponse {
        project_id: params.project_id,
        result,
        state_root,
        proofs,
        ledger_seq: 1234567, // Simulated ledger sequence
    })
}
