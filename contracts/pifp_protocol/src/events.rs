#![allow(deprecated)]

use soroban_sdk::{contractevent, contracttype, symbol_short, Address, BytesN, Env};

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

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectCancelled {
    pub project_id: u64,
    pub cancelled_by: Address,
}

#[contractevent]
pub struct FundsReleased {
    pub project_id: u64,
    pub token: Address,
    pub amount: i128,
}

#[contractevent]
/// Structured refund event data (previously emitted as a bare tuple).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Refunded {
    pub project_id: u64,
    pub donator: Address,
    pub amount: i128,
}

#[contractevent]
/// Event data emitted when a creator reclaims unclaimed donor funds
/// after the refund window has expired.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpiredFundsReclaimed {
    pub project_id: u64,
    pub creator: Address,
    pub token: Address,
    pub amount: i128,
}

/// Event data for protocol pause / unpause.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolPaused {
    pub admin: Address,
}

#[contractevent]
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolUnpaused {
    pub admin: Address,
}

// ── Emission helpers ────────────────────────────────────────────────

pub fn emit_project_created(
    env: &Env,
    project_id: u64,
    creator: Address,
    token: Address,
    goal: i128,
) {
    ProjectCreated {
        project_id,
        creator,
        token,
        goal,
    }
    .publish(env);
}

pub fn emit_project_funded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    ProjectFunded {
        project_id,
        donator,
        amount,
    }
    .publish(env);
}

pub fn emit_project_active(env: &Env, project_id: u64) {
    ProjectActive { project_id }.publish(env);
}

pub fn emit_project_verified(env: &Env, project_id: u64, oracle: Address, proof_hash: BytesN<32>) {
    ProjectVerified {
        project_id,
        oracle,
        proof_hash,
    }
    .publish(env);
}

pub fn emit_project_expired(env: &Env, project_id: u64, deadline: u64) {
    ProjectExpired {
        project_id,
        deadline,
    }
    .publish(env);
}

pub fn emit_project_cancelled(env: &Env, project_id: u64, cancelled_by: Address) {
    let topics = (symbol_short!("cancelled"), project_id);
    let data = ProjectCancelled {
        project_id,
        cancelled_by,
    };
    env.events().publish(topics, data);
}

pub fn emit_funds_released(env: &Env, project_id: u64, token: Address, amount: i128) {
    FundsReleased {
        project_id,
        token,
        amount,
    }
    .publish(env);
}

pub fn emit_refunded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    Refunded {
        project_id,
        donator,
        amount,
    }
    .publish(env);
}

pub fn emit_protocol_paused(env: &Env, admin: Address) {
    ProtocolPaused { admin }.publish(env);
}

pub fn emit_protocol_unpaused(env: &Env, admin: Address) {
    ProtocolUnpaused { admin }.publish(env);
    let topics = (symbol_short!("refunded"), project_id);
    let data = Refunded {
        project_id,
        donator,
        amount,
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
    let topics = (symbol_short!("reclaim"), project_id, token.clone());
    let data = ExpiredFundsReclaimed {
        project_id,
        creator,
        token,
        amount,
    };
    env.events().publish(topics, data);
}

pub fn emit_protocol_paused(env: &Env, admin: Address) {
    let topics = (symbol_short!("paused"), admin.clone());
    let data = ProtocolPaused { admin };
    env.events().publish(topics, data);
}

pub fn emit_protocol_unpaused(env: &Env, admin: Address) {
    let topics = (symbol_short!("unpaused"), admin.clone());
    let data = ProtocolUnpaused { admin };
    env.events().publish(topics, data);
}

pub fn emit_deadline_extended(
    env: &Env,
    project_id: u64,
    old_deadline: u64,
    new_deadline: u64,
) {
    let topics = (symbol_short!("ext_dead"), project_id);
    let data = DeadlineExtended {
        project_id,
        old_deadline,
        new_deadline,
    };
    env.events().publish(topics, data);
}

pub fn emit_protocol_config_updated(
    env: &Env,
    old_config: Option<ProtocolConfig>,
    new_config: ProtocolConfig,
) {
    let topics = (symbol_short!("cfg_upd"),);
    let data = ProtocolConfigUpdated {
        old_fee_recipient: old_config.as_ref().map(|c| c.fee_recipient.clone()),
        old_fee_bps: old_config.map(|c| c.fee_bps).unwrap_or(0),
        new_fee_recipient: new_config.fee_recipient,
        new_fee_bps: new_config.fee_bps,
    };
    env.events().publish(topics, data);
}

pub fn emit_fee_deducted(env: &Env, project_id: u64, token: Address, amount: i128, recipient: Address) {
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
    let topics = (symbol_short!("wl_add"), project_id);
    let data = WhitelistAdded {
        project_id,
        address,
    };
    env.events().publish(topics, data);
}

pub fn emit_whitelist_removed(env: &Env, project_id: u64, address: Address) {
    let topics = (symbol_short!("wl_rem"), project_id);
    let data = WhitelistRemoved {
        project_id,
        address,
    };
    env.events().publish(topics, data);
}
