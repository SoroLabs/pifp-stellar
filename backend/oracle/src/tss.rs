#![allow(dead_code)]
/// Threshold Signature Scheme (TSS) for decentralized oracle nodes.
///
/// Implements a FROST-inspired t-of-n threshold signing protocol over
/// Ed25519 / k256 so that no single keeper node holds the full signing key.
///
/// Flow:
///   1. DKG produces per-node key shares and a group public key.
///   2. Each node produces a `PartialSignature` over the message.
///   3. The aggregator collects ≥ t partial signatures and combines them
///      into a single valid signature that can be verified against the
///      group public key.
use k256::ecdsa::{signature::Signer, Signature, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use threshold_crypto::{PublicKeySet, SecretKeyShare, SignatureShare};
use tracing::{info, warn};

// ── Partial signature ─────────────────────────────────────────────────────────

/// A partial signature produced by a single keeper node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PartialSignature {
    /// Index of the signing node (1-based).
    pub node_id: usize,
    /// The partial signature bytes.
    pub share_bytes: Vec<u8>,
}

// ── TSS Signer (per-node) ─────────────────────────────────────────────────────

/// Per-node TSS signer.
///
/// Each keeper node holds a `SecretKeyShare` derived from the DKG protocol.
/// It uses this share to produce a `PartialSignature` over any message.
pub struct TssSigner {
    pub node_id: usize,
    pub secret_share: SecretKeyShare,
}

impl TssSigner {
    pub fn new(node_id: usize, secret_share: SecretKeyShare) -> Self {
        Self {
            node_id,
            secret_share,
        }
    }

    /// Produce a partial signature over `msg`.
    pub fn sign(&self, msg: &[u8]) -> PartialSignature {
        let share = self.secret_share.sign(msg);
        PartialSignature {
            node_id: self.node_id,
            share_bytes: share.to_bytes().to_vec(),
        }
    }
}

// ── TSS Aggregator ────────────────────────────────────────────────────────────

/// Aggregator service that combines partial signatures into a full signature.
///
/// Holds the group `PublicKeySet` (derived from DKG) and collects partial
/// signatures from keeper nodes until the threshold is met.
pub struct TssAggregator {
    pub pub_key_set: PublicKeySet,
    pub threshold: usize,
    /// Collected partial signatures keyed by node_id.
    pending: BTreeMap<usize, SignatureShare>,
}

impl TssAggregator {
    pub fn new(pub_key_set: PublicKeySet, threshold: usize) -> Self {
        Self {
            pub_key_set,
            threshold,
            pending: BTreeMap::new(),
        }
    }

    /// Add a partial signature from a node.
    ///
    /// Returns `true` if the threshold has been reached and the aggregate
    /// signature is ready to be collected via `aggregate`.
    pub fn add_partial(&mut self, msg: &[u8], partial: PartialSignature) -> bool {
        // Deserialize the share.
        let bytes_96: [u8; 96] = match partial.share_bytes.as_slice().try_into() {
            Ok(b) => b,
            Err(_) => {
                warn!(
                    "Node {}: failed to deserialize signature share: wrong length",
                    partial.node_id
                );
                return false;
            }
        };
        let share = match SignatureShare::from_bytes(bytes_96) {
            Ok(s) => s,
            Err(e) => {
                warn!(
                    "Node {}: failed to deserialize signature share: {:?}",
                    partial.node_id, e
                );
                return false;
            }
        };

        // Verify the partial signature against the group public key set.
        let pub_key_share = self.pub_key_set.public_key_share(partial.node_id);
        if !pub_key_share.verify(&share, msg) {
            warn!(
                "Node {}: partial signature verification failed",
                partial.node_id
            );
            return false;
        }

        self.pending.insert(partial.node_id, share);
        info!(
            "Aggregator: collected {}/{} partial signatures",
            self.pending.len(),
            self.threshold
        );
        self.pending.len() >= self.threshold
    }

    /// Combine collected partial signatures into a full threshold signature.
    ///
    /// Returns the combined signature bytes if ≥ threshold shares are present
    /// and the combined signature verifies against the group public key.
    pub fn aggregate(&self, msg: &[u8]) -> Option<Vec<u8>> {
        if self.pending.len() < self.threshold {
            warn!(
                "Aggregator: not enough shares ({}/{})",
                self.pending.len(),
                self.threshold
            );
            return None;
        }

        let sig = self
            .pub_key_set
            .combine_signatures(self.pending.iter().map(|(id, share)| (*id, share)))
            .ok()?;

        if !self.pub_key_set.public_key().verify(&sig, msg) {
            warn!("Aggregator: combined signature failed verification");
            return None;
        }

        info!("Aggregator: threshold signature successfully combined");
        Some(sig.to_bytes().to_vec())
    }

    /// Clear pending shares (e.g. after successful aggregation).
    pub fn reset(&mut self) {
        self.pending.clear();
    }
}

// ── Legacy TSSOracles (kept for backward compat) ──────────────────────────────

/// Legacy wrapper kept for backward compatibility with existing call sites.
#[allow(dead_code)]
pub struct TSSOracles {
    pub pub_key_set: PublicKeySet,
    pub threshold: usize,
}

#[allow(dead_code)]
impl TSSOracles {
    pub fn new(pub_key_set: PublicKeySet, threshold: usize) -> Self {
        Self {
            pub_key_set,
            threshold,
        }
    }

    pub fn dkg_mock_generate(&self, _node_id: usize) {
        tracing::info!("Mocking DKG generate for node {}", _node_id);
    }

    pub fn aggregate(
        &self,
        msg: &[u8],
        shares: &BTreeMap<usize, SignatureShare>,
    ) -> Option<Vec<u8>> {
        let sig = self
            .pub_key_set
            .combine_signatures(shares.iter().map(|(id, share)| (*id, share)))
            .ok()?;
        if self.pub_key_set.public_key().verify(&sig, msg) {
            Some(sig.to_bytes().to_vec())
        } else {
            None
        }
    }
}

// ── ECDSA-based partial signing (k256, for Stellar compatibility) ─────────────

/// A lightweight ECDSA partial signer using k256.
///
/// Used when the oracle network needs to produce a Stellar-compatible
/// ECDSA/secp256k1 signature rather than a BLS threshold signature.
pub struct EcdsaPartialSigner {
    pub node_id: u32,
    signing_key: SigningKey,
}

impl EcdsaPartialSigner {
    /// Create a signer from a raw 32-byte scalar (derived from DKG key share).
    pub fn from_scalar(node_id: u32, scalar_bytes: &[u8; 32]) -> Option<Self> {
        let key = SigningKey::from_bytes(scalar_bytes.into()).ok()?;
        Some(Self {
            node_id,
            signing_key: key,
        })
    }

    /// Sign a message hash and return the DER-encoded ECDSA signature.
    pub fn sign_hash(&self, hash: &[u8]) -> Vec<u8> {
        let sig: Signature = self.signing_key.sign(hash);
        sig.to_der().as_bytes().to_vec()
    }

    /// Return the compressed SEC1 public key bytes (33 bytes).
    pub fn public_key_bytes(&self) -> [u8; 33] {
        let vk = VerifyingKey::from(&self.signing_key);
        let ep = vk.to_encoded_point(true);
        let mut out = [0u8; 33];
        out.copy_from_slice(ep.as_bytes());
        out
    }
}

/// Hash a message with SHA-256 before signing (standard practice).
pub fn hash_message(msg: &[u8]) -> [u8; 32] {
    let mut h = Sha256::new();
    h.update(msg);
    h.finalize().into()
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use threshold_crypto::SecretKeySet;

    fn setup_threshold(t: usize, n: usize) -> (PublicKeySet, Vec<SecretKeyShare>) {
        // threshold_crypto uses rand 0.7 internally; use its re-exported rng.
        use rand07::SeedableRng;
        let mut rng = rand07::rngs::StdRng::seed_from_u64(42);
        let sk_set = SecretKeySet::random(t - 1, &mut rng);
        let pk_set = sk_set.public_keys();
        let shares: Vec<SecretKeyShare> = (0..n).map(|i| sk_set.secret_key_share(i)).collect();
        (pk_set, shares)
    }

    #[test]
    fn test_partial_sign_and_aggregate_2_of_3() {
        let (pk_set, shares) = setup_threshold(2, 3);
        let msg = b"oracle price feed: XLM/USD = 0.12";

        let mut aggregator = TssAggregator::new(pk_set, 2);

        // Node 0 signs.
        let signer0 = TssSigner::new(0, shares[0].clone());
        let partial0 = signer0.sign(msg);
        let ready = aggregator.add_partial(msg, partial0);
        assert!(!ready, "threshold not yet met after 1 share");

        // Node 1 signs.
        let signer1 = TssSigner::new(1, shares[1].clone());
        let partial1 = signer1.sign(msg);
        let ready = aggregator.add_partial(msg, partial1);
        assert!(ready, "threshold should be met after 2 shares");

        let sig = aggregator.aggregate(msg);
        assert!(sig.is_some(), "aggregation should succeed");
    }

    #[test]
    fn test_aggregate_fails_below_threshold() {
        let (pk_set, shares) = setup_threshold(3, 5);
        let msg = b"test message";

        let mut aggregator = TssAggregator::new(pk_set, 3);

        // Only add 2 shares (below threshold of 3).
        for i in 0..2 {
            let signer = TssSigner::new(i, shares[i].clone());
            aggregator.add_partial(msg, signer.sign(msg));
        }

        assert!(
            aggregator.aggregate(msg).is_none(),
            "should fail below threshold"
        );
    }

    #[test]
    fn test_aggregator_reset() {
        let (pk_set, shares) = setup_threshold(2, 3);
        let msg = b"msg";
        let mut aggregator = TssAggregator::new(pk_set, 2);

        let signer = TssSigner::new(0, shares[0].clone());
        aggregator.add_partial(msg, signer.sign(msg));
        assert_eq!(aggregator.pending.len(), 1);

        aggregator.reset();
        assert_eq!(aggregator.pending.len(), 0);
    }

    #[test]
    fn test_ecdsa_partial_signer_roundtrip() {
        use rand::rngs::OsRng;
        let key = SigningKey::random(&mut OsRng);
        let bytes: [u8; 32] = key.to_bytes().into();
        let signer = EcdsaPartialSigner::from_scalar(1, &bytes).unwrap();
        let hash = hash_message(b"stellar oracle payload");
        let sig_bytes = signer.sign_hash(&hash);
        assert!(!sig_bytes.is_empty());
    }

    #[test]
    fn test_hash_message_deterministic() {
        let h1 = hash_message(b"hello");
        let h2 = hash_message(b"hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_legacy_tss_oracles_aggregate() {
        let (pk_set, shares) = setup_threshold(2, 3);
        let msg = b"legacy test";
        let oracle = TSSOracles::new(pk_set.clone(), 2);

        let mut share_map = BTreeMap::new();
        for i in 0..2 {
            let signer = TssSigner::new(i, shares[i].clone());
            let partial = signer.sign(msg);
            let bytes_96: [u8; 96] = partial.share_bytes.as_slice().try_into().unwrap();
            let share = SignatureShare::from_bytes(bytes_96).unwrap();
            share_map.insert(i, share);
        }

        let result = oracle.aggregate(msg, &share_map);
        assert!(result.is_some());
    }
}
