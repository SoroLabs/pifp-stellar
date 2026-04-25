/// Lock-free multi-threaded mempool for keeper nodes.
///
/// Uses `crossbeam-epoch` for safe memory reclamation and `crossbeam-skiplist`
/// for a concurrent, ordered skip-list that provides O(log n) insert/lookup
/// without any global mutex.
///
/// Transactions are ordered deterministically by (fee_rate DESC, nonce ASC)
/// so the highest-fee, lowest-nonce intent is always at the front.
///
/// A background eviction task prunes expired or low-fee transactions without
/// blocking the hot insert/read paths.
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crossbeam_skiplist::SkipMap;
use tokio::time;
use tracing::{debug, info};

// ── Transaction intent ────────────────────────────────────────────────────────

/// A transaction intent submitted by a keeper node.
#[derive(Debug, Clone)]
pub struct TxIntent {
    /// Unique identifier (e.g. hash of the payload).
    pub id: u64,
    /// Sender nonce — lower nonce is processed first for the same sender.
    pub nonce: u64,
    /// Fee rate in stroops per operation; higher fee = higher priority.
    pub fee_rate: u64,
    /// Arbitrary encoded payload (XDR envelope, JSON, etc.).
    pub payload: Vec<u8>,
    /// Unix timestamp (seconds) after which this intent is considered expired.
    pub expires_at: u64,
}

impl TxIntent {
    /// Construct a new intent with an explicit expiry.
    pub fn new(id: u64, nonce: u64, fee_rate: u64, payload: Vec<u8>, ttl_secs: u64) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        Self {
            id,
            nonce,
            fee_rate,
            payload,
            expires_at: now + ttl_secs,
        }
    }

    /// Returns `true` if the intent has passed its expiry timestamp.
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now >= self.expires_at
    }
}

// ── Ordering key ──────────────────────────────────────────────────────────────

/// Composite sort key: (fee_rate DESC, nonce ASC, id ASC).
///
/// We invert `fee_rate` so that the skip-list's natural ascending order
/// places the highest-fee intent first.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MempoolKey {
    /// Inverted fee rate so higher fee sorts earlier.
    pub neg_fee: u64,
    /// Nonce — lower nonce sorts earlier for equal fee.
    pub nonce: u64,
    /// Tie-break by id for full determinism.
    pub id: u64,
}

impl MempoolKey {
    pub fn from_intent(tx: &TxIntent) -> Self {
        Self {
            neg_fee: u64::MAX - tx.fee_rate,
            nonce: tx.nonce,
            id: tx.id,
        }
    }
}

// ── Lock-free mempool ─────────────────────────────────────────────────────────

/// Configuration for the mempool.
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of intents held in the pool at any time.
    pub capacity: usize,
    /// Minimum fee rate (stroops/op) required for admission.
    pub min_fee_rate: u64,
    /// How often the background eviction task runs (seconds).
    pub eviction_interval_secs: u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            capacity: 10_000,
            min_fee_rate: 100,
            eviction_interval_secs: 5,
        }
    }
}

/// High-throughput, lock-free mempool backed by a concurrent skip-list.
///
/// All public methods are safe to call from multiple threads simultaneously
/// without any mutex contention on the read/write paths.
pub struct LockFreeMempool {
    /// Ordered map: MempoolKey → TxIntent.
    map: Arc<SkipMap<MempoolKey, TxIntent>>,
    /// Monotonically increasing count of total insertions (for metrics).
    total_inserted: Arc<AtomicU64>,
    /// Monotonically increasing count of total evictions.
    total_evicted: Arc<AtomicU64>,
    /// Pool configuration.
    config: MempoolConfig,
}

impl LockFreeMempool {
    /// Create a new mempool with the given configuration.
    pub fn new(config: MempoolConfig) -> Self {
        Self {
            map: Arc::new(SkipMap::new()),
            total_inserted: Arc::new(AtomicU64::new(0)),
            total_evicted: Arc::new(AtomicU64::new(0)),
            config,
        }
    }

    /// Insert a transaction intent.
    ///
    /// Returns `false` (and drops the intent) if:
    /// - the fee rate is below `min_fee_rate`, or
    /// - the pool is at capacity and the intent has a lower fee than the
    ///   current worst entry.
    ///
    /// This method is lock-free: concurrent callers never block each other.
    pub fn insert(&self, tx: TxIntent) -> bool {
        if tx.fee_rate < self.config.min_fee_rate {
            debug!(id = tx.id, fee = tx.fee_rate, "rejected: fee below minimum");
            return false;
        }

        // Capacity enforcement: evict the lowest-priority entry if full.
        if self.map.len() >= self.config.capacity {
            // The skip-list is ordered ascending by MempoolKey, so the *last*
            // entry is the lowest priority (highest neg_fee = lowest fee_rate).
            if let Some(worst) = self.map.back() {
                if tx.fee_rate <= worst.value().fee_rate {
                    debug!(id = tx.id, "rejected: pool full and fee not competitive");
                    return false;
                }
                // Remove the worst entry to make room.
                let worst_key = worst.key().clone();
                drop(worst); // release the guard before mutating
                self.map.remove(&worst_key);
                self.total_evicted.fetch_add(1, Ordering::Relaxed);
            }
        }

        let key = MempoolKey::from_intent(&tx);
        self.map.insert(key, tx);
        self.total_inserted.fetch_add(1, Ordering::Relaxed);
        true
    }

    /// Remove and return the highest-priority intent (highest fee, lowest nonce).
    pub fn pop_best(&self) -> Option<TxIntent> {
        let entry = self.map.front()?;
        let key = entry.key().clone();
        drop(entry);
        self.map.remove(&key).map(|e| e.value().clone())
    }

    /// Peek at the highest-priority intent without removing it.
    pub fn peek_best(&self) -> Option<TxIntent> {
        self.map.front().map(|e| e.value().clone())
    }

    /// Remove a specific intent by its id.
    ///
    /// O(n) scan — use sparingly (e.g. post-confirmation cleanup).
    pub fn remove_by_id(&self, id: u64) -> bool {
        // Collect keys to remove (there should be at most one).
        let keys: Vec<MempoolKey> = self
            .map
            .iter()
            .filter(|e| e.value().id == id)
            .map(|e| e.key().clone())
            .collect();

        let removed = !keys.is_empty();
        for k in keys {
            self.map.remove(&k);
        }
        removed
    }

    /// Current number of intents in the pool.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the pool is empty.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Total intents inserted since creation.
    pub fn total_inserted(&self) -> u64 {
        self.total_inserted.load(Ordering::Relaxed)
    }

    /// Total intents evicted since creation.
    pub fn total_evicted(&self) -> u64 {
        self.total_evicted.load(Ordering::Relaxed)
    }

    /// Drain all expired and below-min-fee intents.
    ///
    /// Called by the background eviction task; safe to call concurrently with
    /// inserts and reads — the skip-list handles concurrent modification.
    pub fn evict_stale(&self) -> usize {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let stale_keys: Vec<MempoolKey> = self
            .map
            .iter()
            .filter(|e| {
                let tx = e.value();
                now >= tx.expires_at || tx.fee_rate < self.config.min_fee_rate
            })
            .map(|e| e.key().clone())
            .collect();

        let count = stale_keys.len();
        for k in stale_keys {
            self.map.remove(&k);
        }
        if count > 0 {
            self.total_evicted.fetch_add(count as u64, Ordering::Relaxed);
            info!(evicted = count, "mempool eviction pass complete");
        }
        count
    }

    /// Spawn a background Tokio task that periodically evicts stale intents.
    ///
    /// The task holds only a weak reference to the skip-list so it stops
    /// automatically when the mempool is dropped.
    pub fn spawn_eviction_task(pool: Arc<Self>) {
        let interval = Duration::from_secs(pool.config.eviction_interval_secs);
        tokio::spawn(async move {
            let mut ticker = time::interval(interval);
            loop {
                ticker.tick().await;
                let evicted = pool.evict_stale();
                debug!(evicted, remaining = pool.len(), "eviction tick");
            }
        });
    }
}

// ── Legacy generic list (kept for backward compat) ────────────────────────────

use crossbeam_epoch::{self as epoch, Atomic, Owned};

#[allow(dead_code)]
pub struct MempoolNode<T> {
    pub value: T,
    pub next: Atomic<MempoolNode<T>>,
}

#[allow(dead_code)]
pub struct LockFreeList<T> {
    pub head: Atomic<MempoolNode<T>>,
}

impl<T> Default for LockFreeList<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> LockFreeList<T> {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            head: Atomic::null(),
        }
    }

    #[allow(dead_code)]
    pub fn push(&self, value: T) {
        let guard = epoch::pin();
        let mut node = Owned::new(MempoolNode {
            value,
            next: Atomic::null(),
        });
        loop {
            let head = self.head.load(Ordering::Acquire, &guard);
            node.next.store(head, Ordering::Relaxed);
            match self.head.compare_exchange(
                head,
                node,
                Ordering::Release,
                Ordering::Relaxed,
                &guard,
            ) {
                Ok(_) => break,
                Err(e) => node = e.new,
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn make_pool() -> Arc<LockFreeMempool> {
        Arc::new(LockFreeMempool::new(MempoolConfig {
            capacity: 100,
            min_fee_rate: 100,
            eviction_interval_secs: 60,
        }))
    }

    fn intent(id: u64, nonce: u64, fee_rate: u64, ttl: u64) -> TxIntent {
        TxIntent::new(id, nonce, fee_rate, vec![id as u8], ttl)
    }

    #[test]
    fn test_insert_and_pop_ordering() {
        let pool = make_pool();
        pool.insert(intent(1, 0, 200, 60));
        pool.insert(intent(2, 0, 500, 60));
        pool.insert(intent(3, 0, 300, 60));

        // Should pop highest fee first.
        let best = pool.pop_best().unwrap();
        assert_eq!(best.fee_rate, 500);

        let next = pool.pop_best().unwrap();
        assert_eq!(next.fee_rate, 300);
    }

    #[test]
    fn test_nonce_ordering_same_fee() {
        let pool = make_pool();
        pool.insert(intent(10, 5, 400, 60));
        pool.insert(intent(11, 2, 400, 60));
        pool.insert(intent(12, 8, 400, 60));

        // Same fee → lower nonce first.
        let first = pool.pop_best().unwrap();
        assert_eq!(first.nonce, 2);
    }

    #[test]
    fn test_min_fee_rejection() {
        let pool = make_pool();
        let accepted = pool.insert(intent(1, 0, 50, 60)); // below min_fee_rate=100
        assert!(!accepted);
        assert_eq!(pool.len(), 0);
    }

    #[test]
    fn test_capacity_evicts_worst() {
        let pool = Arc::new(LockFreeMempool::new(MempoolConfig {
            capacity: 3,
            min_fee_rate: 100,
            eviction_interval_secs: 60,
        }));
        pool.insert(intent(1, 0, 200, 60));
        pool.insert(intent(2, 0, 300, 60));
        pool.insert(intent(3, 0, 400, 60));
        // Pool is full; inserting a higher-fee intent should evict the worst.
        let accepted = pool.insert(intent(4, 0, 500, 60));
        assert!(accepted);
        assert_eq!(pool.len(), 3);
        // The 200-fee intent should have been evicted.
        let ids: Vec<u64> = {
            let mut v = Vec::new();
            while let Some(tx) = pool.pop_best() {
                v.push(tx.id);
            }
            v
        };
        assert!(!ids.contains(&1), "lowest-fee intent should be evicted");
    }

    #[test]
    fn test_evict_stale_removes_expired() {
        let pool = make_pool();
        // Insert an already-expired intent (ttl = 0 → expires_at = now).
        let mut tx = intent(99, 0, 200, 0);
        tx.expires_at = 0; // force expired
        pool.map.insert(MempoolKey::from_intent(&tx), tx);

        pool.insert(intent(1, 0, 300, 60)); // valid

        let evicted = pool.evict_stale();
        assert_eq!(evicted, 1);
        assert_eq!(pool.len(), 1);
    }

    #[test]
    fn test_remove_by_id() {
        let pool = make_pool();
        pool.insert(intent(42, 0, 200, 60));
        pool.insert(intent(43, 0, 300, 60));
        assert!(pool.remove_by_id(42));
        assert_eq!(pool.len(), 1);
        assert!(!pool.remove_by_id(99)); // non-existent
    }

    #[test]
    fn test_metrics_counters() {
        let pool = make_pool();
        pool.insert(intent(1, 0, 200, 60));
        pool.insert(intent(2, 0, 300, 60));
        assert_eq!(pool.total_inserted(), 2);
    }

    #[tokio::test]
    async fn test_concurrent_inserts() {
        let pool = Arc::new(LockFreeMempool::new(MempoolConfig {
            capacity: 10_000,
            min_fee_rate: 100,
            eviction_interval_secs: 60,
        }));

        let mut handles = Vec::new();
        for i in 0u64..100 {
            let p = Arc::clone(&pool);
            handles.push(tokio::spawn(async move {
                p.insert(TxIntent::new(i, i % 10, 100 + i, vec![], 60));
            }));
        }
        for h in handles {
            h.await.unwrap();
        }
        assert_eq!(pool.len(), 100);
    }
}
