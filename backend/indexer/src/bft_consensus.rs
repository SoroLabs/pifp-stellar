//! Byzantine Fault Tolerant (BFT) Consensus Layer for Keeper Nodes (Issue #261)
//!
//! Implements a lightweight 3-phase commit (Propose → Prevote → Precommit)
//! consensus protocol tolerant of up to ⌊(n-1)/3⌋ faulty or offline nodes,
//! with VRF-based leader election to prevent targeted DoS.

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tracing::{debug, info, warn};

// ─── Types ───────────────────────────────────────────────────────────────────

/// A node identity (public key fingerprint, 32 bytes hex).
pub type NodeId = String;
/// An opaque application command to be replicated.
pub type Command = Vec<u8>;
/// Consensus round number.
pub type Round = u64;
/// Cryptographic signature (stub – production should use ed25519-dalek).
pub type Signature = Vec<u8>;

// ─── VRF-Based Leader Election ────────────────────────────────────────────────

/// A minimal VRF output used to elect the round leader unpredictably.
/// Production should use a proper VRF (e.g., ed25519-vrf or ECVRF).
#[derive(Debug, Clone)]
pub struct VrfOutput {
    /// Deterministic pseudo-random value derived from `seed || node_id`.
    pub value: [u8; 32],
    /// Proof that `value` was honestly computed (stub: SHA-256 hash).
    pub proof: Vec<u8>,
}

impl VrfOutput {
    /// Evaluate VRF: H(seed || node_id).  Replace with a real VRF in production.
    pub fn evaluate(seed: &[u8], node_id: &NodeId) -> Self {
        let mut h = Sha256::new();
        h.update(seed);
        h.update(node_id.as_bytes());
        let value: [u8; 32] = h.finalize().into();
        Self { value, proof: value.to_vec() }
    }

    /// Numeric score derived from the VRF output (lower wins in this election).
    pub fn score(&self) -> u128 {
        u128::from_le_bytes(self.value[..16].try_into().unwrap())
    }
}

/// Elect the leader for `round` from `peers` using VRF outputs.
/// The node with the lowest VRF score becomes the proposer.
pub fn elect_leader(round: Round, peers: &[NodeId]) -> Option<NodeId> {
    let seed = round.to_le_bytes();
    peers
        .iter()
        .min_by_key(|id| VrfOutput::evaluate(&seed, id).score())
        .cloned()
}

// ─── State Machine Interface ──────────────────────────────────────────────────

/// Any application that wants BFT replication must implement this trait.
pub trait StateMachine: Send + Sync + 'static {
    /// Apply a committed command and return the new state hash.
    fn apply(&mut self, cmd: &Command) -> [u8; 32];
    /// Return the current state hash for cross-node verification.
    fn state_hash(&self) -> [u8; 32];
}

// ─── Message Types ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Phase {
    Propose,
    Prevote,
    Precommit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMessage {
    pub phase: Phase,
    pub round: Round,
    /// SHA-256 digest of the proposed command.
    pub value_hash: [u8; 32],
    pub sender: NodeId,
    /// Signature over `phase || round || value_hash` (stub).
    pub signature: Signature,
}

impl ConsensusMessage {
    pub fn new(phase: Phase, round: Round, value_hash: [u8; 32], sender: NodeId) -> Self {
        // Stub signature: hash of the message fields.
        let mut h = Sha256::new();
        h.update(format!("{:?}{round}", phase).as_bytes());
        h.update(value_hash);
        h.update(sender.as_bytes());
        let signature = h.finalize().to_vec();
        Self { phase, round, value_hash, sender, signature }
    }

    /// Verify the stub signature (replace with ed25519 in production).
    pub fn verify(&self) -> bool {
        let mut h = Sha256::new();
        h.update(format!("{:?}{}", self.phase, self.round).as_bytes());
        h.update(self.value_hash);
        h.update(self.sender.as_bytes());
        h.finalize().as_slice() == self.signature.as_slice()
    }
}

// ─── Consensus Engine ────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
enum EnginePhase {
    Idle,
    Proposing,
    Prevoting,
    Precommitting,
    Committed,
}

struct RoundState {
    round: Round,
    phase: EnginePhase,
    proposed_hash: Option<[u8; 32]>,
    proposed_cmd: Option<Command>,
    prevotes: HashMap<NodeId, [u8; 32]>,
    precommits: HashMap<NodeId, [u8; 32]>,
    started_at: Instant,
}

impl RoundState {
    fn new(round: Round) -> Self {
        Self {
            round,
            phase: EnginePhase::Idle,
            proposed_hash: None,
            proposed_cmd: None,
            prevotes: HashMap::new(),
            precommits: HashMap::new(),
            started_at: Instant::now(),
        }
    }
}

/// Core BFT consensus engine.
pub struct BftConsensus<SM: StateMachine> {
    pub node_id: NodeId,
    peers: Vec<NodeId>,
    state_machine: Arc<Mutex<SM>>,
    current_round: Arc<Mutex<RoundState>>,
    /// Committed value hashes (for idempotency).
    committed: Arc<Mutex<HashSet<[u8; 32]>>>,
    /// Round timeout – if no commit within this window, advance to next round.
    round_timeout: Duration,
}

impl<SM: StateMachine> BftConsensus<SM> {
    /// Create a new consensus engine.
    ///
    /// `peers` should include `node_id` itself.
    pub fn new(node_id: NodeId, peers: Vec<NodeId>, state_machine: SM) -> Self {
        Self {
            node_id,
            peers,
            state_machine: Arc::new(Mutex::new(state_machine)),
            current_round: Arc::new(Mutex::new(RoundState::new(1))),
            committed: Arc::new(Mutex::new(HashSet::new())),
            round_timeout: Duration::from_secs(5),
        }
    }

    /// Quorum size: ⌊2n/3⌋ + 1  (tolerates up to ⌊(n-1)/3⌋ failures).
    fn quorum(&self) -> usize {
        2 * self.peers.len() / 3 + 1
    }

    /// Whether this node is the elected leader for the given round.
    pub fn is_leader(&self, round: Round) -> bool {
        elect_leader(round, &self.peers).as_deref() == Some(&self.node_id)
    }

    /// **Phase 1 – Propose**: The leader broadcasts a proposed command.
    pub fn propose(&self, cmd: Command) -> Option<ConsensusMessage> {
        let mut rs = self.current_round.lock().unwrap();
        if !self.is_leader(rs.round) {
            warn!("propose called on non-leader node {}", self.node_id);
            return None;
        }
        let hash = Self::hash_cmd(&cmd);
        rs.proposed_cmd = Some(cmd);
        rs.proposed_hash = Some(hash);
        rs.phase = EnginePhase::Proposing;
        let msg = ConsensusMessage::new(Phase::Propose, rs.round, hash, self.node_id.clone());
        info!(round = rs.round, node = %self.node_id, "PROPOSE sent");
        Some(msg)
    }

    /// **Phase 2 – Prevote**: On receiving a valid Propose, broadcast a prevote.
    pub fn handle_propose(&self, msg: &ConsensusMessage) -> Option<ConsensusMessage> {
        if !msg.verify() {
            warn!("invalid signature on PROPOSE from {}", msg.sender);
            return None;
        }
        let mut rs = self.current_round.lock().unwrap();
        if msg.round != rs.round {
            return None;
        }
        rs.proposed_hash = Some(msg.value_hash);
        rs.phase = EnginePhase::Prevoting;
        let prevote = ConsensusMessage::new(Phase::Prevote, rs.round, msg.value_hash, self.node_id.clone());
        debug!(round = rs.round, node = %self.node_id, "PREVOTE sent");
        Some(prevote)
    }

    /// **Phase 3 – Precommit**: On receiving a quorum of prevotes, broadcast precommit.
    pub fn handle_prevote(&self, msg: &ConsensusMessage) -> Option<ConsensusMessage> {
        if !msg.verify() {
            return None;
        }
        let mut rs = self.current_round.lock().unwrap();
        if msg.round != rs.round || msg.phase != Phase::Prevote {
            return None;
        }
        rs.prevotes.insert(msg.sender.clone(), msg.value_hash);
        if rs.prevotes.len() >= self.quorum() && rs.phase == EnginePhase::Prevoting {
            rs.phase = EnginePhase::Precommitting;
            let hash = rs.proposed_hash?;
            let pc = ConsensusMessage::new(Phase::Precommit, rs.round, hash, self.node_id.clone());
            debug!(round = rs.round, node = %self.node_id, "PRECOMMIT sent");
            return Some(pc);
        }
        None
    }

    /// **Commit**: On receiving a quorum of precommits, apply to the state machine.
    /// Returns the new state hash on success.
    pub fn handle_precommit(&self, msg: &ConsensusMessage, cmd: Option<&Command>) -> Option<[u8; 32]> {
        if !msg.verify() {
            return None;
        }
        let mut rs = self.current_round.lock().unwrap();
        if msg.round != rs.round || msg.phase != Phase::Precommit {
            return None;
        }
        rs.precommits.insert(msg.sender.clone(), msg.value_hash);
        if rs.precommits.len() >= self.quorum() && rs.phase == EnginePhase::Precommitting {
            let hash = rs.proposed_hash?;
            let mut committed = self.committed.lock().unwrap();
            if committed.contains(&hash) {
                // Idempotent.
                return Some(hash);
            }
            let apply_cmd = cmd.or_else(|| rs.proposed_cmd.as_ref());
            if let Some(c) = apply_cmd {
                let new_hash = self.state_machine.lock().unwrap().apply(c);
                committed.insert(hash);
                rs.phase = EnginePhase::Committed;
                info!(round = rs.round, node = %self.node_id, state_hash = hex::encode(new_hash), "COMMITTED");
                // Advance to the next round.
                let next_round = rs.round + 1;
                drop(rs);
                *self.current_round.lock().unwrap() = RoundState::new(next_round);
                return Some(new_hash);
            }
        }
        None
    }

    /// Check for a round timeout and advance if needed.
    pub fn tick(&self) {
        let rs = self.current_round.lock().unwrap();
        if rs.started_at.elapsed() > self.round_timeout && rs.phase != EnginePhase::Committed {
            let next = rs.round + 1;
            warn!(round = rs.round, "round timeout — advancing to round {next}");
            drop(rs);
            *self.current_round.lock().unwrap() = RoundState::new(next);
        }
    }

    fn hash_cmd(cmd: &Command) -> [u8; 32] {
        let mut h = Sha256::new();
        h.update(cmd);
        h.finalize().into()
    }

    pub fn current_round(&self) -> Round {
        self.current_round.lock().unwrap().round
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    struct MockSM {
        state: Vec<u8>,
    }

    impl StateMachine for MockSM {
        fn apply(&mut self, cmd: &Command) -> [u8; 32] {
            self.state.extend_from_slice(cmd);
            let mut h = Sha256::new();
            h.update(&self.state);
            h.finalize().into()
        }
        fn state_hash(&self) -> [u8; 32] {
            let mut h = Sha256::new();
            h.update(&self.state);
            h.finalize().into()
        }
    }

    fn make_cluster(n: usize) -> (Vec<BftConsensus<MockSM>>, Vec<NodeId>) {
        let ids: Vec<NodeId> = (0..n).map(|i| format!("node{i:02}")).collect();
        let nodes = ids
            .iter()
            .map(|id| {
                BftConsensus::new(
                    id.clone(),
                    ids.clone(),
                    MockSM { state: Vec::new() },
                )
            })
            .collect();
        (nodes, ids)
    }

    #[test]
    fn leader_elected_deterministically() {
        let peers: Vec<NodeId> = (0..7).map(|i| format!("node{i}")).collect();
        let l1 = elect_leader(1, &peers);
        let l2 = elect_leader(1, &peers);
        assert_eq!(l1, l2, "leader election must be deterministic");
    }

    #[test]
    fn three_phase_commit_succeeds_with_quorum() {
        let (nodes, ids) = make_cluster(4);

        // Find the leader for round 1.
        let leader_id = elect_leader(1, &ids).unwrap();
        let leader_idx = ids.iter().position(|id| id == &leader_id).unwrap();

        let cmd = b"transfer 100 XLM".to_vec();

        // Phase 1: leader proposes.
        let propose_msg = nodes[leader_idx].propose(cmd.clone()).unwrap();
        assert_eq!(propose_msg.phase, Phase::Propose);

        // Phase 2: all nodes prevote.
        let prevote_msgs: Vec<_> = nodes
            .iter()
            .filter_map(|n| n.handle_propose(&propose_msg))
            .collect();
        assert_eq!(prevote_msgs.len(), 4);

        // Phase 3: collect prevotes → precommits.
        let mut precommit_msgs: Vec<ConsensusMessage> = Vec::new();
        for node in &nodes {
            for pv in &prevote_msgs {
                if let Some(pc) = node.handle_prevote(pv) {
                    precommit_msgs.push(pc);
                    break; // one precommit per node
                }
            }
        }

        // Commit: feed precommits back.
        let mut committed_hashes: Vec<[u8; 32]> = Vec::new();
        for node in &nodes {
            for pc in &precommit_msgs {
                if let Some(h) = node.handle_precommit(pc, Some(&cmd)) {
                    committed_hashes.push(h);
                    break;
                }
            }
        }
        assert!(!committed_hashes.is_empty(), "at least one node must commit");
        // All committed hashes must match.
        assert!(committed_hashes.windows(2).all(|w| w[0] == w[1]));
    }

    #[test]
    fn message_signature_verification() {
        let msg = ConsensusMessage::new(Phase::Prevote, 5, [0u8; 32], "nodeA".to_string());
        assert!(msg.verify());
    }
}
