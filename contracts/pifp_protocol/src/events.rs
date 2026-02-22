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
pub struct ProjectVerified {
    pub project_id: u64,
    pub oracle: Address,
    pub proof_hash: BytesN<32>,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FundsReleased {
    pub project_id: u64,
    pub token: Address,
    pub amount: i128,
}

pub fn emit_project_created(env: &Env, project_id: u64, creator: Address, token: Address, goal: i128) {
pub fn emit_project_created(
    env: &Env,
    project_id: u64,
    creator: Address,
    token: Address,
    goal: i128,
) {
    let topics = (symbol_short!("created"), project_id);
    let data = ProjectCreated {
        project_id,
        creator,
        token,
        goal,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_funded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    let topics = (symbol_short!("funded"), project_id);
    let data = ProjectFunded {
        project_id,
        donator,
        amount,
    };
    env.events().publish(topics, data);
}

pub fn emit_project_verified(env: &Env, project_id: u64, oracle: Address, proof_hash: BytesN<32>) {
    let topics = (symbol_short!("verified"), project_id);
    let data = ProjectVerified {
        project_id,
        oracle,
        proof_hash,
    };
    env.events().publish(topics, data);
}

pub fn emit_funds_released(env: &Env, project_id: u64, token: Address, amount: i128) {
    let topics = (symbol_short!("released"), project_id, token.clone());
    let data = FundsReleased {
        project_id,
        token,
        amount,
    };
    env.events().publish(topics, data);
pub fn emit_protocol_paused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("paused"), admin), ());
}

pub fn emit_protocol_unpaused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("unpaused"), admin), ());
}
