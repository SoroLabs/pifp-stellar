//! On-chain event definitions and emission helpers for the PIFP protocol.

use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

use crate::types::ProtocolConfig;

// ── Event data structs ────────────────────────────────────────────────────────

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCreated {
    pub project_id: u64,
    pub creator: Address,
    pub token: Address,
    pub goal: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectFunded {
    pub project_id: u64,
    pub donator: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectActive {
    pub project_id: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectVerified {
    pub project_id: u64,
    pub oracle: Address,
    pub proof_hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectExpired {
    pub project_id: u64,
    pub deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCancelled {
    pub project_id: u64,
    pub cancelled_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsReleased {
    pub project_id: u64,
    pub token: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    pub project_id: u64,
    pub donator: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpiredFundsReclaimed {
    pub project_id: u64,
    pub creator: Address,
    pub token: Address,
    pub amount: i128,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolPaused {
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolUnpaused {
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeadlineExtended {
    pub project_id: u64,
    pub old_deadline: u64,
    pub new_deadline: u64,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfigUpdated {
    pub old_fee_recipient: Option<Address>,
    pub old_fee_bps: u32,
    pub new_fee_recipient: Address,
    pub new_fee_bps: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeeDeducted {
    pub project_id: u64,
    pub token: Address,
    pub amount: i128,
    pub recipient: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhitelistAdded {
    pub project_id: u64,
    pub address: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WhitelistRemoved {
    pub project_id: u64,
    pub address: Address,
}

/// Emitted each time an oracle casts a vote via `verify_and_release`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleVoted {
    pub project_id: u64,
    pub oracle: Address,
    /// Bit index of this oracle in the project's authorized list.
    pub oracle_index: u32,
    /// Running count of unique votes after this one.
    pub voter_count: u32,
    /// Threshold required to release funds.
    pub threshold: u32,
}

/// Emitted when an oracle is added to a project's authorized list.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleAdded {
    pub project_id: u64,
    pub oracle: Address,
}

/// Emitted when an oracle is removed from a project's authorized list.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleRemoved {
    pub project_id: u64,
    pub oracle: Address,
}

// ── Emission helpers ──────────────────────────────────────────────────────────

pub fn emit_project_created(env: &Env, project_id: u64, creator: Address, token: Address, goal: i128) {
    let topics = (symbol_short!("created"), project_id);
    env.events().publish(topics, ProjectCreated { project_id, creator, token, goal });
}

pub fn emit_project_funded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    let topics = (symbol_short!("funded"), project_id);
    env.events().publish(topics, ProjectFunded { project_id, donator, amount });
}

pub fn emit_project_active(env: &Env, project_id: u64) {
    let topics = (symbol_short!("active"), project_id);
    env.events().publish(topics, ProjectActive { project_id });
}

pub fn emit_project_verified(env: &Env, project_id: u64, oracle: Address, proof_hash: BytesN<32>) {
    let topics = (symbol_short!("verified"), project_id);
    env.events().publish(topics, ProjectVerified { project_id, oracle, proof_hash });
}

pub fn emit_project_expired(env: &Env, project_id: u64, deadline: u64) {
    let topics = (symbol_short!("expired"), project_id);
    env.events().publish(topics, ProjectExpired { project_id, deadline });
}

pub fn emit_project_cancelled(env: &Env, project_id: u64, cancelled_by: Address) {
    let topics = (symbol_short!("cancelled"), project_id);
    env.events().publish(topics, ProjectCancelled { project_id, cancelled_by });
}

pub fn emit_funds_released(env: &Env, project_id: u64, token: Address, amount: i128) {
    let topics = (symbol_short!("released"), project_id);
    env.events().publish(topics, FundsReleased { project_id, token, amount });
}

pub fn emit_refunded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    let topics = (symbol_short!("refunded"), project_id);
    env.events().publish(topics, Refunded { project_id, donator, amount });
}

pub fn emit_expired_funds_reclaimed(env: &Env, project_id: u64, creator: Address, token: Address, amount: i128) {
    let topics = (symbol_short!("reclaim"), project_id);
    env.events().publish(topics, ExpiredFundsReclaimed { project_id, creator, token, amount });
}

pub fn emit_protocol_paused(env: &Env, admin: Address) {
    let topics = (symbol_short!("paused"),);
    env.events().publish(topics, ProtocolPaused { admin });
}

pub fn emit_protocol_unpaused(env: &Env, admin: Address) {
    let topics = (symbol_short!("unpaused"),);
    env.events().publish(topics, ProtocolUnpaused { admin });
}

pub fn emit_deadline_extended(env: &Env, project_id: u64, old_deadline: u64, new_deadline: u64) {
    let topics = (symbol_short!("ext_dead"), project_id);
    env.events().publish(topics, DeadlineExtended { project_id, old_deadline, new_deadline });
}

pub fn emit_protocol_config_updated(env: &Env, old_config: Option<ProtocolConfig>, new_config: ProtocolConfig) {
    let topics = (symbol_short!("cfg_upd"),);
    env.events().publish(topics, ProtocolConfigUpdated {
        old_fee_recipient: old_config.as_ref().map(|c| c.fee_recipient.clone()),
        old_fee_bps: old_config.map(|c| c.fee_bps).unwrap_or(0),
        new_fee_recipient: new_config.fee_recipient,
        new_fee_bps: new_config.fee_bps,
    });
}

pub fn emit_fee_deducted(env: &Env, project_id: u64, token: Address, amount: i128, recipient: Address) {
    let topics = (symbol_short!("fee_ded"), project_id);
    env.events().publish(topics, FeeDeducted { project_id, token, amount, recipient });
}

pub fn emit_whitelist_added(env: &Env, project_id: u64, address: Address) {
    let topics = (symbol_short!("wl_add"), project_id);
    env.events().publish(topics, WhitelistAdded { project_id, address });
}

pub fn emit_whitelist_removed(env: &Env, project_id: u64, address: Address) {
    let topics = (symbol_short!("wl_rem"), project_id);
    env.events().publish(topics, WhitelistRemoved { project_id, address });
}

/// Emitted each time an oracle submits a vote (before or at threshold).
pub fn emit_oracle_voted(
    env: &Env,
    project_id: u64,
    oracle: Address,
    oracle_index: u32,
    voter_count: u32,
    threshold: u32,
) {
    let topics = (symbol_short!("orc_vote"), project_id);
    env.events().publish(topics, OracleVoted {
        project_id,
        oracle,
        oracle_index,
        voter_count,
        threshold,
    });
}

/// Emitted when an oracle is added to a project's authorized list.
pub fn emit_oracle_added(env: &Env, project_id: u64, oracle: Address) {
    let topics = (symbol_short!("orc_add"), project_id);
    env.events().publish(topics, OracleAdded { project_id, oracle });
}

/// Emitted when an oracle is removed from a project's authorized list.
pub fn emit_oracle_removed(env: &Env, project_id: u64, oracle: Address) {
    let topics = (symbol_short!("orc_rem"), project_id);
    env.events().publish(topics, OracleRemoved { project_id, oracle });
}
