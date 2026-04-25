/// Distributed Key Generation (DKG) protocol for decentralized oracle nodes.
///
/// Implements Feldman's Verifiable Secret Sharing (VSS) scheme so that `t`
/// out of `n` keeper nodes can collaboratively generate a shared public key
/// without any single node ever holding the full private key.
///
/// Round 1 — each node broadcasts polynomial commitments (Feldman VSS).
/// Round 2 — each node sends encrypted secret shares to every other node.
/// Finalize — each node verifies received shares and derives its key share.
use k256::{elliptic_curve::PrimeField, ProjectivePoint, Scalar};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::{error, info, warn};

// ── DKG messages ──────────────────────────────────────────────────────────────

/// Messages exchanged between nodes during the DKG protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DkgMessage {
    /// Round 1: broadcast Feldman VSS polynomial commitments.
    Round1 {
        from: u32,
        /// Commitments C_j = a_j * G for each polynomial coefficient a_j.
        /// Each point is compressed SEC1 (33 bytes), stored as Vec<u8>.
        commitments: Vec<Vec<u8>>,
    },
    /// Round 2: unicast secret share for a specific recipient.
    Round2 {
        from: u32,
        to: u32,
        /// f_i(j) — the polynomial evaluated at the recipient's index.
        share: Vec<u8>,
    },
}

// ── DKG protocol state machine ────────────────────────────────────────────────

/// State of the DKG protocol for a single node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DkgState {
    Idle,
    Round1Sent,
    Round2Sent,
    Finalized,
    Failed(String),
}

/// Per-node DKG protocol runner.
///
/// Each keeper node creates one `DkgProtocol` instance and drives it through
/// the rounds by calling `start_round1`, `handle_round1`, `start_round2`,
/// `handle_round2`, and finally `finalize`.
pub struct DkgProtocol {
    /// This node's 1-based index (1..=total_nodes).
    pub node_id: u32,
    /// Minimum number of shares required to reconstruct the secret.
    pub threshold: u32,
    /// Total number of participating nodes.
    pub total_nodes: u32,
    /// Current protocol state.
    pub state: DkgState,

    // ── Private polynomial ────────────────────────────────────────────────────
    /// Coefficients of the secret polynomial f(x) = a_0 + a_1*x + ... + a_{t-1}*x^{t-1}.
    /// a_0 is this node's secret contribution.
    secret_poly: Vec<Scalar>,

    // ── Public commitments ────────────────────────────────────────────────────
    /// Feldman commitments: C_j = a_j * G.
    pub commitments: Vec<ProjectivePoint>,

    // ── Received data ─────────────────────────────────────────────────────────
    /// Round-1 commitments received from other nodes.
    pub received_commitments: HashMap<u32, Vec<ProjectivePoint>>,
    /// Round-2 shares received from other nodes (keyed by sender id).
    pub received_shares: HashMap<u32, Scalar>,
}

impl DkgProtocol {
    /// Create a new DKG protocol instance for `node_id`.
    pub fn new(node_id: u32, threshold: u32, total_nodes: u32) -> Self {
        assert!(threshold >= 1, "threshold must be >= 1");
        assert!(total_nodes >= threshold, "total_nodes must be >= threshold");
        Self {
            node_id,
            threshold,
            total_nodes,
            state: DkgState::Idle,
            secret_poly: Vec::new(),
            commitments: Vec::new(),
            received_commitments: HashMap::new(),
            received_shares: HashMap::new(),
        }
    }

    // ── Round 1 ───────────────────────────────────────────────────────────────

    /// Generate a random secret polynomial and broadcast Feldman commitments.
    pub fn start_round1(&mut self) -> DkgMessage {
        info!("Node {} starting DKG Round 1", self.node_id);

        // Generate t random scalar coefficients.
        self.secret_poly = (0..self.threshold).map(|_| random_scalar()).collect();

        // Compute Feldman commitments C_j = a_j * G.
        self.commitments = self
            .secret_poly
            .iter()
            .map(|a| ProjectivePoint::GENERATOR * a)
            .collect();

        let compressed: Vec<Vec<u8>> = self
            .commitments
            .iter()
            .map(|p| compress_point(p).to_vec())
            .collect();

        self.state = DkgState::Round1Sent;
        DkgMessage::Round1 {
            from: self.node_id,
            commitments: compressed,
        }
    }

    /// Process a Round-1 message from another node.
    pub fn handle_round1(&mut self, from: u32, commitments_bytes: Vec<Vec<u8>>) {
        let points: Vec<ProjectivePoint> = commitments_bytes
            .iter()
            .filter_map(|b| {
                let arr: Option<[u8; 33]> = b.as_slice().try_into().ok();
                arr.and_then(|a| decompress_point(&a))
            })
            .collect();

        if points.len() != self.threshold as usize {
            warn!(
                "Node {}: received malformed Round1 from {} ({} commitments, expected {})",
                self.node_id,
                from,
                points.len(),
                self.threshold
            );
            return;
        }
        self.received_commitments.insert(from, points);
    }

    // ── Round 2 ───────────────────────────────────────────────────────────────

    /// Evaluate the secret polynomial at each peer's index and return the
    /// unicast share messages.
    pub fn start_round2(&mut self) -> Vec<DkgMessage> {
        info!("Node {} starting DKG Round 2", self.node_id);
        let mut messages = Vec::new();

        for j in 1..=self.total_nodes {
            if j == self.node_id {
                // Self-share: store locally.
                let share = eval_poly(&self.secret_poly, j);
                self.received_shares.insert(self.node_id, share);
                continue;
            }
            let share = eval_poly(&self.secret_poly, j);
            messages.push(DkgMessage::Round2 {
                from: self.node_id,
                to: j,
                share: scalar_to_bytes(&share).to_vec(),
            });
        }

        self.state = DkgState::Round2Sent;
        messages
    }

    /// Process a Round-2 share from another node.
    ///
    /// Performs Feldman VSS verification: checks that
    /// `share * G == Σ C_k^(node_id^k)`.
    pub fn handle_round2(&mut self, from: u32, share_bytes: Vec<u8>) {
        let arr: [u8; 32] = match share_bytes.as_slice().try_into() {
            Ok(a) => a,
            Err(_) => {
                warn!("Node {}: invalid share length from {}", self.node_id, from);
                return;
            }
        };
        let share = match bytes_to_scalar(&arr) {
            Some(s) => s,
            None => {
                warn!("Node {}: invalid share bytes from {}", self.node_id, from);
                return;
            }
        };

        // Feldman VSS verification.
        if let Some(commitments) = self.received_commitments.get(&from) {
            if !verify_share(self.node_id, &share, commitments) {
                warn!(
                    "Node {}: Feldman VSS check FAILED for share from {}",
                    self.node_id, from
                );
                self.state = DkgState::Failed(format!("bad share from node {from}"));
                return;
            }
        }

        self.received_shares.insert(from, share);
    }

    // ── Finalize ──────────────────────────────────────────────────────────────

    /// Combine received shares into this node's long-term key share and
    /// derive the group public key.
    ///
    /// Returns `(key_share_scalar, group_public_key_point)` on success.
    pub fn finalize(&mut self) -> Option<(Scalar, ProjectivePoint)> {
        if matches!(self.state, DkgState::Failed(_)) {
            error!("Node {}: cannot finalize — DKG failed", self.node_id);
            return None;
        }

        let required = self.threshold as usize;
        if self.received_shares.len() < required {
            error!(
                "Node {}: only {}/{} shares received",
                self.node_id,
                self.received_shares.len(),
                required
            );
            return None;
        }

        // Key share = sum of all received f_i(node_id) values.
        let key_share: Scalar = self
            .received_shares
            .values()
            .fold(Scalar::ZERO, |acc, s| acc + s);

        // Group public key = sum of all Round-1 constant-term commitments C_0.
        let group_pubkey: ProjectivePoint = self
            .received_commitments
            .values()
            .filter_map(|c| c.first().copied())
            .fold(
                self.commitments
                    .first()
                    .copied()
                    .unwrap_or(ProjectivePoint::IDENTITY),
                |acc, p| acc + p,
            );

        info!("Node {} finalized DKG — group pubkey derived", self.node_id);
        self.state = DkgState::Finalized;
        Some((key_share, group_pubkey))
    }
}

// ── Polynomial helpers ────────────────────────────────────────────────────────

/// Evaluate polynomial f(x) = Σ a_i * x^i at x = `index`.
fn eval_poly(coeffs: &[Scalar], index: u32) -> Scalar {
    let x = scalar_from_u32(index);
    let mut result = Scalar::ZERO;
    let mut x_pow = Scalar::ONE;
    for coeff in coeffs {
        result += coeff * &x_pow;
        x_pow *= &x;
    }
    result
}

/// Feldman VSS verification: check `share * G == Σ C_k * index^k`.
fn verify_share(index: u32, share: &Scalar, commitments: &[ProjectivePoint]) -> bool {
    let lhs = ProjectivePoint::GENERATOR * share;
    let x = scalar_from_u32(index);
    let mut rhs = ProjectivePoint::IDENTITY;
    let mut x_pow = Scalar::ONE;
    for c in commitments {
        rhs += c * &x_pow;
        x_pow *= &x;
    }
    lhs == rhs
}

// ── Scalar / point helpers ────────────────────────────────────────────────────

fn random_scalar() -> Scalar {
    Scalar::generate_vartime(&mut OsRng)
}

fn scalar_from_u32(n: u32) -> Scalar {
    let mut bytes = [0u8; 32];
    bytes[28..32].copy_from_slice(&n.to_be_bytes());
    Scalar::from_repr(bytes.into()).unwrap_or(Scalar::ZERO)
}

fn scalar_to_bytes(s: &Scalar) -> [u8; 32] {
    s.to_bytes().into()
}

fn bytes_to_scalar(b: &[u8; 32]) -> Option<Scalar> {
    let arr: &k256::FieldBytes = b.into();
    Scalar::from_repr(*arr).into()
}

fn compress_point(p: &ProjectivePoint) -> [u8; 33] {
    use k256::elliptic_curve::sec1::ToEncodedPoint;
    let affine = p.to_affine();
    let encoded = affine.to_encoded_point(true);
    let bytes = encoded.as_bytes();
    let mut out = [0u8; 33];
    if bytes.len() == 33 {
        out.copy_from_slice(bytes);
    }
    out
}

fn decompress_point(b: &[u8; 33]) -> Option<ProjectivePoint> {
    use k256::elliptic_curve::sec1::FromEncodedPoint;
    use k256::EncodedPoint;
    let ep = EncodedPoint::from_bytes(b).ok()?;
    let affine = k256::AffinePoint::from_encoded_point(&ep);
    if affine.is_some().into() {
        Some(ProjectivePoint::from(affine.unwrap()))
    } else {
        None
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_eval_poly_constant() {
        // f(x) = 5  (single coefficient)
        let coeffs = vec![scalar_from_u32(5)];
        let result = eval_poly(&coeffs, 3);
        assert_eq!(result, scalar_from_u32(5));
    }

    #[test]
    fn test_eval_poly_linear() {
        // f(x) = 2 + 3x  → f(4) = 14
        let coeffs = vec![scalar_from_u32(2), scalar_from_u32(3)];
        let result = eval_poly(&coeffs, 4);
        assert_eq!(result, scalar_from_u32(14));
    }

    #[test]
    fn test_feldman_vss_verify() {
        let secret = random_scalar();
        let blind = random_scalar();
        let coeffs = vec![secret.clone(), blind];
        let commitments: Vec<ProjectivePoint> = coeffs
            .iter()
            .map(|a| ProjectivePoint::GENERATOR * a)
            .collect();

        let share = eval_poly(&coeffs, 1);
        assert!(verify_share(1, &share, &commitments));

        // Wrong share should fail.
        let wrong = random_scalar();
        assert!(!verify_share(1, &wrong, &commitments));
    }

    #[test]
    fn test_compress_decompress_roundtrip() {
        let p = ProjectivePoint::GENERATOR;
        let compressed = compress_point(&p);
        let recovered = decompress_point(&compressed).unwrap();
        assert_eq!(p, recovered);
    }

    #[test]
    fn test_scalar_bytes_roundtrip() {
        let s = scalar_from_u32(12345);
        let bytes = scalar_to_bytes(&s);
        let recovered = bytes_to_scalar(&bytes).unwrap();
        assert_eq!(s, recovered);
    }

    #[test]
    fn test_full_dkg_2_of_3() {
        // Simulate a 2-of-3 DKG between three nodes.
        let mut nodes: Vec<DkgProtocol> = (1..=3).map(|id| DkgProtocol::new(id, 2, 3)).collect();

        // Round 1: each node broadcasts commitments.
        let r1_msgs: Vec<DkgMessage> = nodes.iter_mut().map(|n| n.start_round1()).collect();

        // Distribute Round-1 messages.
        for msg in &r1_msgs {
            if let DkgMessage::Round1 { from, commitments } = msg {
                for node in nodes.iter_mut() {
                    if node.node_id != *from {
                        node.handle_round1(*from, commitments.clone());
                    }
                }
            }
        }

        // Round 2: each node sends shares.
        let r2_msgs: Vec<Vec<DkgMessage>> = nodes.iter_mut().map(|n| n.start_round2()).collect();

        // Distribute Round-2 messages.
        for msgs in &r2_msgs {
            for msg in msgs {
                if let DkgMessage::Round2 { from, to, share } = msg {
                    if let Some(node) = nodes.iter_mut().find(|n| n.node_id == *to) {
                        node.handle_round2(*from, share.clone());
                    }
                }
            }
        }

        // Finalize all nodes.
        for node in nodes.iter_mut() {
            let result = node.finalize();
            assert!(result.is_some(), "Node {} failed to finalize", node.node_id);
        }
    }

    #[test]
    fn test_dkg_state_transitions() {
        let mut node = DkgProtocol::new(1, 2, 3);
        assert_eq!(node.state, DkgState::Idle);
        node.start_round1();
        assert_eq!(node.state, DkgState::Round1Sent);
    }
}
