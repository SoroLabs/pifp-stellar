#![allow(deprecated)]

use soroban_sdk::{contractevent, contracttype, symbol_short, Address, BytesN, Env};

#[contractevent]
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
pub struct Refunded {
    pub project_id: u64,
    pub donator: Address,
    pub amount: i128,
}

#[contractevent]
pub struct ProtocolPaused {
    pub admin: Address,
}

#[contractevent]
pub struct ProtocolUnpaused {
    pub admin: Address,
}

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
}
