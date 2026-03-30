//! On-chain event definitions and emission helpers for the PIFP protocol.

use soroban_sdk::{contractevent, contracttype, symbol_short, Address, BytesN, Env};

const PROJECT_CREATED: Symbol = symbol_short!("created");
const FUNDS_RELEASED: Symbol = symbol_short!("released");
const MILESTONE_VERIFIED: Symbol = symbol_short!("m_verify");

#[contractevent]
// ── Event Data Structs ──────────────────────────────────────────────
//
// Each event uses a dedicated struct so that indexers can decode every
// field by name rather than relying on positional tuple elements.
// Topic layout: (event_symbol, project_id) for project-scoped events,
// (event_symbol, caller) for protocol-level events.

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCreated {
    pub project_id: u64,
    pub creator: Address,
    pub token: Address,
    pub goal: i128,
}

#[contractevent]
pub struct ProjectFunded {
    pub project_id: u64,
    pub donator: Address,
    pub amount: i128,
}

#[contractevent]
pub struct ProjectActive {
    pub project_id: u64,
}

#[contractevent]
pub struct ProjectVerified {
    pub project_id: u64,
    pub oracle: Address,
    pub proof_hash: BytesN<32>,
}

#[contractevent]
pub struct ProjectExpired {
    pub project_id: u64,
    pub deadline: u64,
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

use soroban_sdk::{contracttype, symbol_short, Address, BytesN, Env};

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
pub struct ProjectCancelled {
    pub project_id: u64,
    pub cancelled_by: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectPaused {
    pub project_id: u64,
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectUnpaused {
    pub project_id: u64,
    pub admin: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsReleased {
    pub project_id: u64,
    pub oracle: Address,
    /// Bit index of this oracle in the project's authorized list.
    pub oracle_index: u32,
    /// Running count of unique votes after this one.
    pub voter_count: u32,
    /// Threshold required to release funds.
    pub threshold: u32,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleAdded {
    pub project_id: u64,
    pub oracle: Address,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleRemoved {
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

// ── Emission helpers ──────────────────────────────────────────────────────────

pub fn emit_project_created(
    env: &Env,
    project_id: u64,
    creator: Address,
    token: Address,
    goal: i128,
) {
    let topics = (symbol_short!("proj_cr"), project_id);
    let data = ProjectCreated {
        project_id,
        creator,
        token,
        goal,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_funded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    let topics = (symbol_short!("proj_fnd"), project_id);
    let data = ProjectFunded {
        project_id,
        donator,
        amount,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_active(env: &Env, project_id: u64) {
    let topics = (symbol_short!("proj_act"), project_id);
    let data = ProjectActive { project_id };
    env.events().publish(topics, data);
}

pub fn emit_project_verified(env: &Env, project_id: u64, oracle: Address, proof_hash: BytesN<32>) {
    let topics = (symbol_short!("proj_ver"), project_id);
    let data = ProjectVerified {
        project_id,
        oracle,
        proof_hash,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_expired(env: &Env, project_id: u64, deadline: u64) {
    let topics = (symbol_short!("proj_exp"), project_id);
    let data = ProjectExpired {
        project_id,
        deadline,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_cancelled(env: &Env, project_id: u64, cancelled_by: Address) {
    let topics = (symbol_short!("proj_can"), project_id);
    let data = ProjectCancelled {
        project_id,
        cancelled_by,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_paused(env: &Env, project_id: u64, admin: Address) {
    let topics = (symbol_short!("prj_psd"), project_id);
    let data = ProjectPaused { project_id, admin };
    env.events().publish(topics, data);
}

pub fn emit_project_unpaused(env: &Env, project_id: u64, admin: Address) {
    let topics = (symbol_short!("prj_unp"), project_id);
    let data = ProjectUnpaused { project_id, admin };
    env.events().publish(topics, data);
}

pub fn emit_funds_released(env: &Env, project_id: u64, token: Address, amount: i128) {
    let topics = (symbol_short!("fund_rel"), project_id);
    let data = FundsReleased {
        project_id,
        token: token.clone(),
        amount,
    };
    env.events().publish(topics, data);
}

pub fn emit_refunded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    let topics = (symbol_short!("proj_ref"), project_id);
    let data = Refunded {
        project_id,
        donator,
        amount,
    };
    env.events().publish(topics, data);
}

pub fn emit_deadline_extended(env: &Env, project_id: u64, old_deadline: u64, new_deadline: u64) {
    let topics = (symbol_short!("ext_dead"), project_id);
    env.events().publish(topics, DeadlineExtended { project_id, old_deadline, new_deadline });
}

pub fn emit_protocol_config_updated(env: &Env, old_config: Option<ProtocolConfig>, new_config: ProtocolConfig) {
    let topics = (symbol_short!("cfg_upd"),);
    let data = ProtocolConfigUpdated {
        old_fee_recipient: old_config.as_ref().map(|cfg| cfg.fee_recipient.clone()),
        old_fee_bps: old_config.map_or(0, |cfg| cfg.fee_bps),
        new_fee_recipient: new_config.fee_recipient.clone(),
        new_fee_bps: new_config.fee_bps,
    });
}

pub fn emit_fee_deducted(
    env: &Env,
    project_id: u64,
    token: Address,
    amount: i128,
    recipient: Address,
) {
    let topics = (symbol_short!("fee_ded"), project_id, token.clone());
    let data = FeeDeducted {
        project_id,
        token,
        amount,
        recipient,
    };
    env.events().publish(topics, data);
}

pub fn emit_whitelist_added(env: &Env, project_id: u64, address: Address) {
    let topics = (symbol_short!("whl_add"), project_id);
    let data = WhitelistAdded {
        project_id,
        address,
    };
    env.events().publish(topics, data);
}

pub fn emit_whitelist_removed(env: &Env, project_id: u64, address: Address) {
    let topics = (symbol_short!("whl_rem"), project_id);
    let data = WhitelistRemoved {
        project_id,
        address,
    };
    env.events().publish(topics, data);
}

pub fn emit_expired_funds_reclaimed(
    env: &Env,
    project_id: u64,
    creator: Address,
    token: Address,
    amount: i128,
) {
    let topics = (symbol_short!("exp_recl"), project_id);
    let data = ExpiredFundsReclaimed {
        project_id,
        creator,
        token,
        amount,
    };
    env.events().publish(topics, data);
}

pub fn emit_protocol_paused(env: &Env, admin: Address) {
    let topics = (symbol_short!("prot_psd"),);
    let data = ProtocolPaused { admin };
    env.events().publish(topics, data);
}

pub fn emit_protocol_unpaused(env: &Env, admin: Address) {
    let topics = (symbol_short!("prot_unp"),);
    let data = ProtocolUnpaused { admin };
    env.events().publish(topics, data);
}

/// Emitted when a specific milestone is verified and its portion of funds is released.
pub fn emit_milestone_verified(
    env: &Env,
    project_id: u64,
    milestone_index: u32,
    bps: u32,
) {
    let topics = (MILESTONE_VERIFIED, project_id, milestone_index);
    env.events().publish(topics, bps);
}

pub fn emit_project_created(env: &Env, id: u64, creator: Address, token: Address, goal: i128) {
    let topics = (PROJECT_CREATED, id, creator);
    env.events().publish(topics, (token, goal));
}

pub fn emit_funds_released(env: &Env, id: u64, token: Address, amount: i128) {
    let topics = (FUNDS_RELEASED, id, token);
    env.events().publish(topics, amount);
}