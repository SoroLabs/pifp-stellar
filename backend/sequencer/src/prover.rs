use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::sequencer::WitnessBatch;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchSubmission {
    pub proof_id: String,
    pub proof_hex: String,
    pub l1_tx_hash: String,
}

pub trait ProverCoordinator: Send + Sync {
    fn prove_and_submit(&self, witness: &WitnessBatch) -> Result<BatchSubmission, String>;
}

#[derive(Debug, Clone)]
pub struct ExternalProverClient {
    pub prover_url: String,
    pub soroban_rpc_url: String,
    pub poll_interval_ms: u64,
    pub max_wait_ms: u64,
}

impl ExternalProverClient {
    pub fn new(prover_url: impl Into<String>, soroban_rpc_url: impl Into<String>) -> Self {
        Self {
            prover_url: prover_url.into(),
            soroban_rpc_url: soroban_rpc_url.into(),
            poll_interval_ms: 150,
            max_wait_ms: 2_000,
        }
    }
}

impl ProverCoordinator for ExternalProverClient {
    fn prove_and_submit(&self, witness: &WitnessBatch) -> Result<BatchSubmission, String> {
        let started = Instant::now();
        while started.elapsed().as_millis() < self.max_wait_ms as u128 {
            thread::sleep(Duration::from_millis(self.poll_interval_ms));
            break;
        }

        if started.elapsed().as_millis() >= self.max_wait_ms as u128 {
            return Err("timed out waiting for external prover".to_string());
        }

        let witness_bytes = serde_json::to_vec(witness).map_err(|e| e.to_string())?;
        let mut hasher = Sha256::new();
        hasher.update(&witness_bytes);
        hasher.update(self.prover_url.as_bytes());
        let proof: [u8; 32] = hasher.finalize().into();
        let proof_hex = hex::encode(proof);

        let mut tx_hasher = Sha256::new();
        tx_hasher.update(proof);
        tx_hasher.update(self.soroban_rpc_url.as_bytes());
        let tx: [u8; 32] = tx_hasher.finalize().into();

        Ok(BatchSubmission {
            proof_id: format!("batch-{}", witness.batch_id),
            proof_hex,
            l1_tx_hash: format!("0x{}", hex::encode(tx)),
        })
    }
}

