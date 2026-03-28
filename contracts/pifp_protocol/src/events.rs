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
pub struct FundsReleased {
    pub project_id: u64,
    pub token: Address,
    pub amount: i128,
}

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

pub fn emit_project_active(env: &Env, project_id: u64) {
    let topics = (symbol_short!("active"), project_id);
    let data = ProjectActive { project_id };
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

pub fn emit_project_expired(env: &Env, project_id: u64, deadline: u64) {
    let topics = (symbol_short!("expired"), project_id);
    let data = ProjectExpired {
        project_id,
        deadline,
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
}

pub fn emit_refunded(env: &Env, project_id: u64, donator: Address, amount: i128) {
    let topics = (symbol_short!("refunded"), project_id);
    let data = (donator, amount);
    env.events().publish(topics, data);
}

pub fn emit_protocol_paused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("paused"), admin), ());
}

pub fn emit_protocol_unpaused(env: &Env, admin: Address) {
    env.events().publish((symbol_short!("unpaused"), admin), ());
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
