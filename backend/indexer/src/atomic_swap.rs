//! Cross-Chain Atomic Swap Coordinator Daemon (Issue #264)
//!
//! Orchestrates Hashed Time-Lock Contracts (HTLCs) between Stellar/Soroban
//! and an EVM-compatible chain.  Monitors both chains for HTLC lock events,
//! manages cryptographic pre-images, and automatically triggers refunds on
//! timeout.
//!
//! # Architecture
//! ```text
//!  ┌─────────────┐     lock event     ┌──────────────────────┐
//!  │  Soroban    │ ──────────────────► │                      │
//!  │  HTLC ctr   │                    │  AtomicSwapCoordinator│
//!  └─────────────┘                    │                      │
//!  ┌─────────────┐     lock event     │  ┌────────────────┐  │
//!  │  EVM HTLC   │ ──────────────────►│  │ SecretVault    │  │
//!  │  contract   │                    │  └────────────────┘  │
//!  └─────────────┘                    └──────────────────────┘
//! ```

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{error, info, warn};

// ─── Domain Types ─────────────────────────────────────────────────────────────

/// Supported chain identifiers.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Chain {
    Stellar,
    Ethereum,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Stellar => write!(f, "Stellar"),
            Chain::Ethereum => write!(f, "Ethereum"),
        }
    }
}

/// A 32-byte cryptographic secret (pre-image).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Secret(pub [u8; 32]);

impl Secret {
    /// Generate a cryptographically random secret.
    pub fn generate() -> Self {
        // In production replace with `rand::thread_rng().gen()`.
        let mut bytes = [0u8; 32];
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .subsec_nanos();
        // Deterministic but unique-enough for testing; production uses OsRng.
        let mut h = Sha256::new();
        h.update(seed.to_le_bytes());
        h.update(b"pifp-htlc-secret");
        bytes.copy_from_slice(&h.finalize());
        Self(bytes)
    }

    /// Compute the SHA-256 hash-lock (the value committed on-chain).
    pub fn hash_lock(&self) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(self.0);
        h.finalize().into()
    }
}

/// Current lifecycle state of a swap.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SwapStatus {
    /// Coordinator has generated a secret; waiting for initiator to lock funds.
    Initiated,
    /// Funds are locked on the source chain.
    SourceLocked,
    /// Funds are locked on both chains.
    BothLocked,
    /// Secret revealed; funds claimed on destination.
    Completed,
    /// Timeout exceeded; refund broadcast.
    Refunded,
    /// Unrecoverable error state.
    Failed(String),
}

/// An HTLC lock event received from a chain listener.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HtlcLockEvent {
    /// Transaction / contract ID on the originating chain.
    pub tx_id: String,
    pub chain: Chain,
    /// SHA-256 hash-lock that funds are conditional on.
    pub hash_lock: [u8; 32],
    /// Amount in the chain's smallest unit.
    pub amount: u128,
    /// Unix timestamp after which the lock expires and can be refunded.
    pub timeout_ts: u64,
    /// Recipient address on the destination chain.
    pub recipient: String,
}

/// Full record for one atomic swap.
#[derive(Debug, Clone)]
pub struct SwapRecord {
    pub swap_id: String,
    pub status: SwapStatus,
    pub source_chain: Chain,
    pub dest_chain: Chain,
    pub hash_lock: [u8; 32],
    pub initiated_at: u64,
    pub timeout_ts: u64,
    pub source_lock_event: Option<HtlcLockEvent>,
    pub dest_lock_event: Option<HtlcLockEvent>,
}

// ─── Secret Vault ─────────────────────────────────────────────────────────────

/// In-process store for swap secrets.
/// Production should back this with an HSM or encrypted KV store.
#[derive(Default)]
pub struct SecretVault {
    /// Maps `swap_id` → `Secret`
    secrets: HashMap<String, Secret>,
}

impl SecretVault {
    pub fn store(&mut self, swap_id: &str, secret: Secret) {
        self.secrets.insert(swap_id.to_string(), secret);
    }

    pub fn reveal(&self, swap_id: &str) -> Option<&Secret> {
        self.secrets.get(swap_id)
    }

    /// Securely erase the secret after the swap completes.
    pub fn purge(&mut self, swap_id: &str) {
        if let Some(mut s) = self.secrets.remove(swap_id) {
            // Zero-out memory before dropping.
            s.0.iter_mut().for_each(|b| *b = 0);
        }
    }
}

// ─── Chain Listener (trait) ───────────────────────────────────────────────────

/// Trait implemented by each chain's RPC client to poll for HTLC events.
#[async_trait::async_trait]
pub trait ChainListener: Send + Sync {
    fn chain(&self) -> Chain;
    /// Poll the chain for new HTLC lock events since `from_block`.
    async fn poll_lock_events(&self, from_block: u64) -> anyhow::Result<Vec<HtlcLockEvent>>;
    /// Broadcast a claim transaction using `secret`.
    async fn broadcast_claim(&self, event: &HtlcLockEvent, secret: &Secret) -> anyhow::Result<String>;
    /// Broadcast a refund/reclaim transaction after timeout.
    async fn broadcast_refund(&self, event: &HtlcLockEvent) -> anyhow::Result<String>;
}

// ─── Stub Listeners ───────────────────────────────────────────────────────────

/// No-op Soroban listener — replace with real Horizon/RPC polling.
pub struct SorobanListener {
    pub rpc_url: String,
    pub htlc_contract: String,
}

#[async_trait::async_trait]
impl ChainListener for SorobanListener {
    fn chain(&self) -> Chain {
        Chain::Stellar
    }

    async fn poll_lock_events(&self, _from_block: u64) -> anyhow::Result<Vec<HtlcLockEvent>> {
        // TODO: call `stellar_sdk::Server::get_transactions` filtered by `htlc_contract`.
        Ok(vec![])
    }

    async fn broadcast_claim(&self, event: &HtlcLockEvent, secret: &Secret) -> anyhow::Result<String> {
        info!(
            chain = "Stellar",
            tx = event.tx_id,
            "broadcasting HTLC claim with secret 0x{}",
            hex::encode(secret.0)
        );
        // TODO: build & submit Soroban invoke_contract transaction.
        Ok(format!("stub-stellar-claim-{}", event.tx_id))
    }

    async fn broadcast_refund(&self, event: &HtlcLockEvent) -> anyhow::Result<String> {
        warn!(chain = "Stellar", tx = event.tx_id, "broadcasting HTLC refund (timeout)");
        Ok(format!("stub-stellar-refund-{}", event.tx_id))
    }
}

/// No-op EVM listener — replace with real ethers-rs / alloy polling.
pub struct EvmListener {
    pub rpc_url: String,
    pub htlc_contract: String,
}

#[async_trait::async_trait]
impl ChainListener for EvmListener {
    fn chain(&self) -> Chain {
        Chain::Ethereum
    }

    async fn poll_lock_events(&self, _from_block: u64) -> anyhow::Result<Vec<HtlcLockEvent>> {
        // TODO: call eth_getLogs for `HTLCLocked` events.
        Ok(vec![])
    }

    async fn broadcast_claim(&self, event: &HtlcLockEvent, secret: &Secret) -> anyhow::Result<String> {
        info!(
            chain = "Ethereum",
            tx = event.tx_id,
            "broadcasting HTLC claim with secret 0x{}",
            hex::encode(secret.0)
        );
        Ok(format!("stub-evm-claim-{}", event.tx_id))
    }

    async fn broadcast_refund(&self, event: &HtlcLockEvent) -> anyhow::Result<String> {
        warn!(chain = "Ethereum", tx = event.tx_id, "broadcasting HTLC refund (timeout)");
        Ok(format!("stub-evm-refund-{}", event.tx_id))
    }
}

// ─── Coordinator ──────────────────────────────────────────────────────────────

/// The main daemon that orchestrates atomic swaps across chains.
pub struct AtomicSwapCoordinator {
    listeners: Vec<Arc<dyn ChainListener>>,
    vault: Arc<Mutex<SecretVault>>,
    swaps: Arc<Mutex<HashMap<String, SwapRecord>>>,
    /// How often to poll each chain for new events.
    poll_interval: Duration,
}

impl AtomicSwapCoordinator {
    pub fn new(listeners: Vec<Arc<dyn ChainListener>>, poll_interval: Duration) -> Self {
        Self {
            listeners,
            vault: Arc::new(Mutex::new(SecretVault::default())),
            swaps: Arc::new(Mutex::new(HashMap::new())),
            poll_interval,
        }
    }

    // ── Public API ──────────────────────────────────────────────────────────

    /// Initiate a new swap: generate a secret and register the swap record.
    pub fn initiate_swap(
        &self,
        swap_id: &str,
        source_chain: Chain,
        dest_chain: Chain,
        timeout_secs: u64,
    ) -> [u8; 32] {
        let secret = Secret::generate();
        let hash_lock = secret.hash_lock();
        self.vault.lock().unwrap().store(swap_id, secret);

        let now = now_ts();
        let record = SwapRecord {
            swap_id: swap_id.to_string(),
            status: SwapStatus::Initiated,
            source_chain,
            dest_chain,
            hash_lock,
            initiated_at: now,
            timeout_ts: now + timeout_secs,
            source_lock_event: None,
            dest_lock_event: None,
        };
        self.swaps.lock().unwrap().insert(swap_id.to_string(), record);
        info!(swap_id, hash_lock = hex::encode(hash_lock), "swap initiated");
        hash_lock
    }

    /// Process an incoming HTLC lock event from any chain.
    pub async fn handle_lock_event(&self, event: HtlcLockEvent) {
        let swap_id = self.find_swap_by_hash_lock(event.hash_lock);
        let Some(swap_id) = swap_id else {
            warn!(tx = event.tx_id, "no swap found for hash_lock, ignoring");
            return;
        };

        let new_status = {
            let mut swaps = self.swaps.lock().unwrap();
            let rec = swaps.get_mut(&swap_id).unwrap();

            match event.chain {
                ref c if *c == rec.source_chain => {
                    info!(swap_id, chain = %event.chain, "source chain locked");
                    rec.source_lock_event = Some(event.clone());
                    if rec.dest_lock_event.is_some() {
                        SwapStatus::BothLocked
                    } else {
                        SwapStatus::SourceLocked
                    }
                }
                _ => {
                    info!(swap_id, chain = %event.chain, "destination chain locked");
                    rec.dest_lock_event = Some(event.clone());
                    if rec.source_lock_event.is_some() {
                        SwapStatus::BothLocked
                    } else {
                        SwapStatus::SourceLocked
                    }
                }
            }
        };

        self.set_status(&swap_id, new_status.clone());

        // If both sides are locked, reveal the secret and claim.
        if new_status == SwapStatus::BothLocked {
            self.claim_on_dest_chain(&swap_id).await;
        }
    }

    /// Background tick: check for expired swaps and broadcast refunds.
    pub async fn tick_refunds(&self) {
        let expired: Vec<(String, HtlcLockEvent)> = {
            let swaps = self.swaps.lock().unwrap();
            swaps
                .values()
                .filter(|r| {
                    matches!(r.status, SwapStatus::Initiated | SwapStatus::SourceLocked)
                        && now_ts() >= r.timeout_ts
                })
                .filter_map(|r| {
                    r.source_lock_event
                        .clone()
                        .map(|ev| (r.swap_id.clone(), ev))
                })
                .collect()
        };

        for (swap_id, event) in expired {
            info!(swap_id, "timeout reached — broadcasting refund");
            let listener = self.listener_for_chain(&event.chain);
            if let Some(l) = listener {
                match l.broadcast_refund(&event).await {
                    Ok(tx) => {
                        info!(swap_id, refund_tx = tx, "refund broadcast");
                        self.set_status(&swap_id, SwapStatus::Refunded);
                        self.vault.lock().unwrap().purge(&swap_id);
                    }
                    Err(e) => {
                        error!(swap_id, "refund broadcast failed: {e}");
                        self.set_status(&swap_id, SwapStatus::Failed(e.to_string()));
                    }
                }
            }
        }
    }

    pub fn swap_status(&self, swap_id: &str) -> Option<SwapStatus> {
        self.swaps.lock().unwrap().get(swap_id).map(|r| r.status.clone())
    }

    // ── Internal helpers ────────────────────────────────────────────────────

    async fn claim_on_dest_chain(&self, swap_id: &str) {
        let (dest_event, dest_chain) = {
            let swaps = self.swaps.lock().unwrap();
            let rec = swaps.get(swap_id).unwrap();
            (rec.dest_lock_event.clone(), rec.dest_chain.clone())
        };
        let Some(event) = dest_event else { return };
        let secret = self.vault.lock().unwrap().reveal(swap_id).cloned();
        let Some(secret) = secret else {
            error!(swap_id, "secret not found — cannot claim");
            return;
        };
        let listener = self.listener_for_chain(&dest_chain);
        if let Some(l) = listener {
            match l.broadcast_claim(&event, &secret).await {
                Ok(tx) => {
                    info!(swap_id, claim_tx = tx, "claim broadcast — swap complete");
                    self.set_status(swap_id, SwapStatus::Completed);
                    self.vault.lock().unwrap().purge(swap_id);
                }
                Err(e) => {
                    error!(swap_id, "claim broadcast failed: {e}");
                    self.set_status(swap_id, SwapStatus::Failed(e.to_string()));
                }
            }
        }
    }

    fn listener_for_chain(&self, chain: &Chain) -> Option<Arc<dyn ChainListener>> {
        self.listeners.iter().find(|l| &l.chain() == chain).cloned()
    }

    fn find_swap_by_hash_lock(&self, hash_lock: [u8; 32]) -> Option<String> {
        self.swaps
            .lock()
            .unwrap()
            .values()
            .find(|r| r.hash_lock == hash_lock)
            .map(|r| r.swap_id.clone())
    }

    fn set_status(&self, swap_id: &str, status: SwapStatus) {
        if let Some(rec) = self.swaps.lock().unwrap().get_mut(swap_id) {
            rec.status = status;
        }
    }

    /// Start the daemon loop (runs until the process exits).
    pub async fn run(self: Arc<Self>) {
        info!("AtomicSwapCoordinator daemon started");
        loop {
            self.tick_refunds().await;
            tokio::time::sleep(self.poll_interval).await;
        }
    }
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn secret_hash_lock_roundtrip() {
        let s = Secret::generate();
        let h = s.hash_lock();
        // hash_lock must be the SHA-256 of the secret
        let mut hasher = Sha256::new();
        hasher.update(s.0);
        let expected: [u8; 32] = hasher.finalize().into();
        assert_eq!(h, expected);
    }

    #[test]
    fn vault_store_and_reveal() {
        let mut vault = SecretVault::default();
        let secret = Secret::generate();
        let original = secret.0;
        vault.store("swap1", secret);
        let revealed = vault.reveal("swap1").unwrap();
        assert_eq!(revealed.0, original);
    }

    #[test]
    fn vault_purge_removes_secret() {
        let mut vault = SecretVault::default();
        vault.store("swap2", Secret::generate());
        vault.purge("swap2");
        assert!(vault.reveal("swap2").is_none());
    }

    #[test]
    fn initiate_swap_returns_correct_hash_lock() {
        let coordinator = AtomicSwapCoordinator::new(vec![], Duration::from_secs(1));
        let hash_lock = coordinator.initiate_swap("swapA", Chain::Stellar, Chain::Ethereum, 3600);

        let vault = coordinator.vault.lock().unwrap();
        let secret = vault.reveal("swapA").unwrap();
        assert_eq!(secret.hash_lock(), hash_lock);
    }

    #[test]
    fn initiate_swap_status_is_initiated() {
        let coordinator = AtomicSwapCoordinator::new(vec![], Duration::from_secs(1));
        coordinator.initiate_swap("swapB", Chain::Stellar, Chain::Ethereum, 3600);
        assert_eq!(coordinator.swap_status("swapB"), Some(SwapStatus::Initiated));
    }

    #[tokio::test]
    async fn handle_source_lock_event_updates_status() {
        let coordinator = AtomicSwapCoordinator::new(vec![], Duration::from_secs(1));
        let hash_lock = coordinator.initiate_swap("swapC", Chain::Stellar, Chain::Ethereum, 3600);

        let event = HtlcLockEvent {
            tx_id: "stellar-tx-001".to_string(),
            chain: Chain::Stellar,
            hash_lock,
            amount: 1_000_000,
            timeout_ts: now_ts() + 3600,
            recipient: "0xABCD".to_string(),
        };
        coordinator.handle_lock_event(event).await;
        assert_eq!(coordinator.swap_status("swapC"), Some(SwapStatus::SourceLocked));
    }

    #[tokio::test]
    async fn expired_swap_with_no_lock_does_not_refund() {
        let coordinator = AtomicSwapCoordinator::new(vec![], Duration::from_secs(1));
        // Initiate but set timeout in the past (swap stays Initiated, no lock event recorded).
        coordinator.initiate_swap("swapD", Chain::Stellar, Chain::Ethereum, 0);
        // tick_refunds should handle gracefully even with no source_lock_event.
        coordinator.tick_refunds().await;
        // Status stays Initiated because there's no lock event to refund.
        assert_eq!(coordinator.swap_status("swapD"), Some(SwapStatus::Initiated));
    }
}
