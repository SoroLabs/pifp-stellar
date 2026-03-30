//! # PIFP Protocol Contract
//!
//! Proof-of-Impact Funding Protocol — Soroban smart contract.
//!
//! | Phase        | Entry Point(s)                                          |
//! |--------------|---------------------------------------------------------|
//! | Bootstrap    | [`PifpProtocol::init`]                                  |
//! | Role admin   | `grant_role`, `revoke_role`, `transfer_super_admin`     |
//! | Oracle mgmt  | `add_oracle`, `remove_oracle`, `set_oracle`             |
//! | Registration | [`PifpProtocol::register_project`]                      |
//! | Funding      | [`PifpProtocol::deposit`]                               |
//! | Donor safety | [`PifpProtocol::refund`]                                |
//! | Verification | [`PifpProtocol::verify_proof`]                          |
//! | Claiming     | [`PifpProtocol::claim_funds`]                           |
//! | Queries      | `get_project`, `get_project_balances`, `role_of`, etc.  |

#![no_std]

use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Bytes, BytesN, Env, Vec,
};

/// Refund window: 6 months after a project enters a terminal refundable state.
pub const REFUND_WINDOW: u64 = 6 * 30 * 24 * 60 * 60;

/// Grace period: 24 hours (in seconds) between proof verification and fund
/// release, allowing community disputes.
const GRACE_PERIOD: u64 = 24 * 60 * 60; // 86_400 seconds

/// Maximum allowed length for a project metadata URI / CID.
const MAX_METADATA_URI_LEN: u32 = 64;

/// Maximum number of authorized oracles per project (fits in a u32 BitSet).
const MAX_ORACLES: u32 = 32;

pub mod errors;
pub mod events;
pub mod invariants_checker;
pub mod rbac;
pub mod categories;
mod storage;
mod types;
mod milestones; // Added module

#[cfg(test)]
mod fuzz_test;
#[cfg(test)]
mod rbac_test;
#[cfg(test)]
mod test;
#[cfg(test)]
mod test_deadline;
#[cfg(test)]
mod test_donation_count;
#[cfg(test)]
mod test_errors;
#[cfg(test)]
mod test_events;
#[cfg(test)]
mod test_expire;
#[cfg(test)]
mod test_project_pause;
#[cfg(test)]
mod test_protocol_config;
#[cfg(test)]
mod test_reclaim;
#[cfg(test)]
mod test_refund;
#[cfg(test)]
mod test_utils;
#[cfg(test)]
mod test_whitelist;
#[cfg(test)]
mod test_grace_period;
#[cfg(test)]
mod test_batch_deposit;

use crate::types::ProjectStatus;
pub use errors::Error;
pub use events::emit_funds_released;
pub use rbac::Role;
use storage::{
    clear_oracle_agreement, drain_token_balance, get_all_balances, get_and_increment_project_id,
    get_protocol_config, is_whitelisted, load_oracle_agreement, load_project, load_project_pair,
    maybe_load_project, save_oracle_agreement, save_project, save_project_config,
    save_project_state, set_protocol_config,
};
pub use types::{
    DepositRequest, Milestone, OracleAgreement, Project, ProjectBalances, ProjectConfig,
    ProjectState, ProtocolConfig,
};

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {
    // ─────────────────────────────────────────────────────────
    // Initialisation
    // ─────────────────────────────────────────────────────────

    pub fn init(env: Env, super_admin: Address) {
        super_admin.require_auth();
        rbac::init_super_admin(&env, &super_admin);
    }

    // ─────────────────────────────────────────────────────────
    // Role management
    // ─────────────────────────────────────────────────────────

    pub fn grant_role(env: Env, caller: Address, target: Address, role: Role) {
        rbac::grant_role(&env, &caller, &target, role);
    }

    pub fn revoke_role(env: Env, caller: Address, target: Address) {
        rbac::revoke_role(&env, &caller, &target);
    }

    pub fn transfer_super_admin(env: Env, current_super_admin: Address, new_super_admin: Address) {
        rbac::transfer_super_admin(&env, &current_super_admin, &new_super_admin);
    }

    pub fn role_of(env: Env, address: Address) -> Option<Role> {
        rbac::role_of(&env, address)
    }

    pub fn has_role(env: Env, address: Address, role: Role) -> bool {
        rbac::has_role(&env, address, role)
    }

    // ─────────────────────────────────────────────────────────
    // Emergency Control
    // ─────────────────────────────────────────────────────────

    pub fn pause(env: Env, caller: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        storage::set_paused(&env, true);
        events::emit_protocol_paused(&env, caller);
    }

    pub fn unpause(env: Env, caller: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        storage::set_paused(&env, false);
        events::emit_protocol_unpaused(&env, caller);
    }

    pub fn is_paused(env: Env) -> bool {
        storage::is_paused(&env)
    }

    // ─────────────────────────────────────────────────────────
    // Oracle management
    // ─────────────────────────────────────────────────────────

    pub fn add_oracle(env: Env, admin: Address, project_id: u64, oracle: Address) {
        admin.require_auth();
        rbac::require_admin_or_above(&env, &admin);

        let mut config = storage::load_project_config(&env, project_id);
        if config.authorized_oracles.len() >= MAX_ORACLES {
            panic_with_error!(&env, Error::InvalidOracleConfig);
        }

        for existing in config.authorized_oracles.iter() {
            if existing == oracle { return; }
        }

        config.authorized_oracles.push_back(oracle.clone());
        save_project_config(&env, project_id, &config);
        clear_oracle_agreement(&env, project_id);
        events::emit_oracle_added(&env, project_id, oracle);
    }

    pub fn remove_oracle(env: Env, admin: Address, project_id: u64, oracle: Address) {
        admin.require_auth();
        rbac::require_admin_or_above(&env, &admin);

        let mut config = storage::load_project_config(&env, project_id);
        let mut found = false;
        let mut new_oracles: Vec<Address> = Vec::new(&env);
        for existing in config.authorized_oracles.iter() {
            if existing == oracle { found = true; }
            else { new_oracles.push_back(existing); }
        }

        if !found { panic_with_error!(&env, Error::NotAuthorized); }

        config.authorized_oracles = new_oracles;
        save_project_config(&env, project_id, &config);
        clear_oracle_agreement(&env, project_id);
        events::emit_oracle_removed(&env, project_id, oracle);
    }

    // ─────────────────────────────────────────────────────────
    // Project lifecycle
    // ─────────────────────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub fn register_project(
        env: Env,
        creator: Address,
        accepted_tokens: Vec<Address>,
        goal: i128,
        proof_hash: BytesN<32>,
        metadata_uri: Bytes,
        deadline: u64,
        is_private: bool,
        milestones: Vec<Milestone>,
        categories: u32,
        authorized_oracles: Vec<Address>,
        threshold: u32,
    ) -> Project {
        Self::require_not_paused(&env);
        creator.require_auth();
        rbac::require_can_register(&env, &creator);

        if milestones.is_empty() { panic_with_error!(&env, Error::InvalidGoal); }
        milestones::validate_milestone_set(&env, &milestones);

        if accepted_tokens.is_empty() || accepted_tokens.len() > 10 {
            panic_with_error!(&env, Error::EmptyAcceptedTokens);
        }
        for i in 0..accepted_tokens.len() {
            let t_i = accepted_tokens.get(i).unwrap();
            if accepted_tokens.last_index_of(&t_i) != Some(i) {
                panic_with_error!(&env, Error::DuplicateToken);
            }
        }
        if goal <= 0 { panic_with_error!(&env, Error::InvalidGoal); }

        let now = env.ledger().timestamp();
        if metadata_uri.is_empty() || metadata_uri.len() > MAX_METADATA_URI_LEN {
            panic_with_error!(&env, Error::MetadataCidInvalid);
        }
        if deadline <= now || deadline > now + 157_680_000 {
            panic_with_error!(&env, Error::InvalidDeadline);
        }

        let oracle_count = authorized_oracles.len();
        if oracle_count > 0 && (threshold == 0 || threshold > oracle_count) {
            panic_with_error!(&env, Error::InvalidOracleConfig);
        }

        let id = get_and_increment_project_id(&env);
        let mut completed_milestones = Vec::new(&env);
        for _ in 0..milestones.len() { completed_milestones.push_back(false); }

        let project = Project {
            id,
            creator: creator.clone(),
            accepted_tokens: accepted_tokens.clone(),
            goal,
            proof_hash,
            metadata_uri: metadata_uri.clone(),
            deadline,
            status: ProjectStatus::Funding,
            donation_count: 0,
            is_private,
            paused: false,
            refund_expiry: 0,
            categories,
            last_proof_time: 0,
            milestones,
            completed_milestones,
            authorized_oracles,
            threshold,
        };

        save_project(&env, &project);
        if let Some(token) = accepted_tokens.get(0) {
            events::emit_project_created(&env, id, creator, token, goal);
        }
        project
    }

    pub fn verify_proof(
        env: Env,
        oracle: Address,
        project_id: u64,
        submitted_proof_hash: BytesN<32>,
    ) {
        Self::require_not_paused(&env);
        oracle.require_auth();

        let (config, mut state) = load_project_pair(&env, project_id);
        Self::require_project_not_paused(&env, &state);

        if env.ledger().timestamp() >= config.deadline {
            state.status = ProjectStatus::Expired;
            state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
            save_project_state(&env, project_id, &state);
            panic_with_error!(&env, Error::ProjectExpired);
        }

        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Verified | ProjectStatus::Completed => panic_with_error!(&env, Error::MilestoneAlreadyReleased),
            _ => panic_with_error!(&env, Error::InvalidTransition),
        }

        if submitted_proof_hash != config.proof_hash {
            panic_with_error!(&env, Error::VerificationFailed);
        }

        if !config.authorized_oracles.is_empty() {
            let mut oracle_index: Option<u32> = None;
            for (i, auth) in config.authorized_oracles.iter().enumerate() {
                if auth == oracle { oracle_index = Some(i as u32); break; }
            }
            let idx = oracle_index.ok_or(Error::NotAuthorized).unwrap();
            let mut agreement = storage::load_oracle_agreement(&env, project_id);
            let bit = 1u32 << idx;
            if (agreement.votes & bit) == 0 {
                agreement.votes |= bit;
                agreement.voter_count += 1;
            }

            if agreement.voter_count < config.threshold {
                storage::save_oracle_agreement(&env, project_id, &agreement);
                return;
            }
            clear_oracle_agreement(&env, project_id);
        } else {
            rbac::require_oracle(&env, &oracle);
        }

        state.status = ProjectStatus::Verified;
        state.last_proof_time = env.ledger().timestamp();
        save_project_state(&env, project_id, &state);
        events::emit_project_verified(&env, project_id, oracle, submitted_proof_hash);
    }

    pub fn claim_funds(env: Env, project_id: u64) {
        Self::require_not_paused(&env);
        let (config, mut state) = load_project_pair(&env, project_id);
        Self::require_project_not_paused(&env, &state);

        if state.status != ProjectStatus::Verified {
            panic_with_error!(&env, Error::InvalidTransition);
        }

        if env.ledger().timestamp() < state.last_proof_time + GRACE_PERIOD {
            panic_with_error!(&env, Error::GracePeriodActive);
        }

        state.status = ProjectStatus::Completed;
        let contract_address = env.current_contract_address();
        let protocol_config = get_protocol_config(&env);

        for token in config.accepted_tokens.iter() {
            let mut balance = drain_token_balance(&env, project_id, &token);
            if balance > 0 {
                let token_client = token::Client::new(&env, &token);
                if let Some(pcfg) = &protocol_config {
                    if pcfg.fee_bps > 0 {
                        let fee = balance.checked_mul(pcfg.fee_bps as i128).unwrap().checked_div(10000).unwrap();
                        if fee > 0 {
                            token_client.transfer(&contract_address, &pcfg.fee_recipient, &fee);
                            balance -= fee;
                            events::emit_fee_deducted(&env, project_id, token.clone(), fee, pcfg.fee_recipient.clone());
                        }
                    }
                }
                if balance > 0 {
                    token_client.transfer(&contract_address, &config.creator, &balance);
                    events::emit_funds_released(&env, project_id, token, balance);
                }
            }
        }
        save_project_state(&env, project_id, &state);
    }

    pub fn deposit(env: Env, project_id: u64, donator: Address, token: Address, amount: i128) {
        Self::require_not_paused(&env);
        donator.require_auth();
        if amount <= 0 { panic_with_error!(&env, Error::InvalidAmount); }

        let (config, mut state) = load_project_pair(&env, project_id);
        Self::require_project_not_paused(&env, &state);

        if env.ledger().timestamp() >= config.deadline {
            if matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active) {
                state.status = ProjectStatus::Expired;
                state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
                save_project_state(&env, project_id, &state);
            }
            panic_with_error!(&env, Error::ProjectExpired);
        }

        if config.is_private && !is_whitelisted(&env, project_id, &donator) {
            panic_with_error!(&env, Error::NotWhitelisted);
        }

        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            _ => panic_with_error!(&env, Error::ProjectNotActive),
        }

        if !config.accepts_token(&token) { panic_with_error!(&env, Error::TokenNotAccepted); }

        let current_donor_balance = storage::get_donator_balance(&env, project_id, &token, &donator);
        if current_donor_balance == 0 {
            state.donation_count += 1;
            save_project_state(&env, project_id, &state);
        }

        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&donator, env.current_contract_address(), &amount);
        let new_balance = storage::add_to_token_balance(&env, project_id, &token, amount);

        if state.status == ProjectStatus::Funding {
            if let Some(first_token) = config.accepted_tokens.get(0) {
                if token == first_token && new_balance >= config.goal {
                    state.status = ProjectStatus::Active;
                    save_project_state(&env, project_id, &state);
                    events::emit_project_active(&env, project_id);
                }
            }
        }

        storage::set_donator_balance(&env, project_id, &token, &donator, current_donor_balance + amount);
        events::emit_project_funded(&env, project_id, donator, amount);
    }

    pub fn batch_deposit(env: Env, donator: Address, deposits: Vec<DepositRequest>) {
        Self::require_not_paused(&env);
        donator.require_auth();
        for req in deposits.iter() {
            Self::deposit(env.clone(), req.project_id, donator.clone(), req.token, req.amount);
        }
    }

    pub fn cancel_project(env: Env, caller: Address, project_id: u64) {
        caller.require_auth();
        rbac::require_can_cancel_project(&env, &caller);
        let (config, mut state) = load_project_pair(&env, project_id);
        Self::require_project_not_paused(&env, &state);

        if state.status != ProjectStatus::Active { panic_with_error!(&env, Error::InvalidTransition); }
        if matches!(rbac::get_role(&env, &caller), Some(Role::ProjectManager)) && caller != config.creator {
             panic_with_error!(&env, Error::NotAuthorized);
        }

        state.status = ProjectStatus::Cancelled;
        state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
        save_project_state(&env, project_id, &state);
        events::emit_project_cancelled(&env, project_id, caller);
    }

    pub fn refund(env: Env, donator: Address, project_id: u64, token: Address) {
        donator.require_auth();
        let (config, mut state) = load_project_pair(&env, project_id);
        if !matches!(state.status, ProjectStatus::Expired | ProjectStatus::Cancelled) {
            panic_with_error!(&env, Error::ProjectNotExpired);
        }
        if state.refund_expiry > 0 && env.ledger().timestamp() >= state.refund_expiry {
            panic_with_error!(&env, Error::RefundWindowExpired);
        }

        let amount = storage::get_donator_balance(&env, project_id, &token, &donator);
        if amount <= 0 { panic_with_error!(&env, Error::InsufficientBalance); }

        storage::set_donator_balance(&env, project_id, &token, &donator, 0);
        storage::add_to_token_balance(&env, project_id, &token, -amount);
        token::Client::new(&env, &token).transfer(&env.current_contract_address(), &donator, &amount);
        events::emit_refunded(&env, project_id, donator, amount);
    }

    pub fn expire_project(env: Env, project_id: u64) {
        let (config, mut state) = load_project_pair(&env, project_id);
        if env.ledger().timestamp() < config.deadline { panic_with_error!(&env, Error::ProjectNotExpired); }
        state.status = ProjectStatus::Expired;
        state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
        save_project_state(&env, project_id, &state);
        events::emit_project_expired(&env, project_id, config.deadline);
    }

    pub fn unpause(env: Env, caller: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        storage::set_paused(&env, false);
        events::emit_protocol_unpaused(&env, caller);
    }

    fn require_not_paused(env: &Env) {
        if storage::is_paused(env) { panic_with_error!(env, Error::ProtocolPaused); }
    }

    fn require_project_not_paused(env: &Env, state: &ProjectState) {
        if state.paused { panic_with_error!(env, Error::ProjectPaused); }
    }
}
