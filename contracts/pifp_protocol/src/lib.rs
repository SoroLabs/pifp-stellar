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
//! | Verification | [`PifpProtocol::verify_and_release`]                    |
//! | Queries      | `get_project`, `get_project_balances`, `role_of`, etc.  |

#![no_std]

use soroban_sdk::{
    contract, contractimpl, panic_with_error, token, Address, Bytes, BytesN, Env, Vec,
};

/// Refund window: 6 months after a project enters a terminal refundable state.
pub const REFUND_WINDOW: u64 = 6 * 30 * 24 * 60 * 60;

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

use crate::types::ProjectStatus;
pub use errors::Error;
pub use events::emit_funds_released;
pub use rbac::Role;
use storage::{
    drain_token_balance, get_all_balances, get_and_increment_project_id, get_protocol_config,
    is_whitelisted, load_project, load_project_pair, maybe_load_project, save_project,
    save_project_config, save_project_state, set_protocol_config,
};
pub use types::{OracleAgreement, Project, ProjectBalances, ProjectConfig, ProjectState, ProjectStatus, ProtocolConfig};

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {
    // ─────────────────────────────────────────────────────────
    // Initialisation
    // ─────────────────────────────────────────────────────────

    /// Initialise the contract and set the first SuperAdmin.
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
    // Oracle management (per-project M-of-N)
    // ─────────────────────────────────────────────────────────

    /// Add an oracle to a project's authorized oracle list.
    ///
    /// - `admin` must hold `SuperAdmin` or `Admin`.
    /// - Maximum 32 oracles per project.
    /// - Adding an oracle resets any in-flight `OracleAgreement` to prevent
    ///   stale bits from a previous oracle at the same index from counting.
    pub fn add_oracle(env: Env, admin: Address, project_id: u64, oracle: Address) {
        admin.require_auth();
        rbac::require_admin_or_above(&env, &admin);

        let mut config = storage::load_project_config(&env, project_id);

        if config.authorized_oracles.len() >= MAX_ORACLES {
            panic_with_error!(&env, Error::InvalidOracleConfig);
        }

        // Idempotent: skip if already present.
        for existing in config.authorized_oracles.iter() {
            if existing == oracle {
                return;
            }
        }

        config.authorized_oracles.push_back(oracle.clone());
        save_project_config(&env, project_id, &config);

        // Reset in-flight agreement — index layout has changed.
        clear_oracle_agreement(&env, project_id);

        events::emit_oracle_added(&env, project_id, oracle);
    }

    /// Remove an oracle from a project's authorized oracle list.
    ///
    /// - `admin` must hold `SuperAdmin` or `Admin`.
    /// - Resets the in-flight `OracleAgreement` so no stale bit remains.
    pub fn remove_oracle(env: Env, admin: Address, project_id: u64, oracle: Address) {
        admin.require_auth();
        rbac::require_admin_or_above(&env, &admin);

        let mut config = storage::load_project_config(&env, project_id);

        let mut found = false;
        let mut new_oracles: Vec<Address> = Vec::new(&env);
        for existing in config.authorized_oracles.iter() {
            if existing == oracle {
                found = true;
            } else {
                new_oracles.push_back(existing);
            }
        }

        if !found {
            panic_with_error!(&env, Error::UnauthorizedOracle);
        }

        config.authorized_oracles = new_oracles;
        save_project_config(&env, project_id, &config);

        // Always reset agreement — bit indices have shifted.
        clear_oracle_agreement(&env, project_id);

        events::emit_oracle_removed(&env, project_id, oracle);
    }

    // ─────────────────────────────────────────────────────────
    // Project lifecycle
    // ─────────────────────────────────────────────────────────

    /// Register a new funding project with M-of-N oracle verification.
    ///
    /// `creator` must hold the `ProjectManager`, `Admin`, or `SuperAdmin` role.
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
        categories: u32,
    ) -> Project {
        Self::require_not_paused(&env);
        creator.require_auth();
        rbac::require_can_register(&env, &creator);

        if accepted_tokens.is_empty() {
            panic_with_error!(&env, Error::EmptyAcceptedTokens);
        }
        if accepted_tokens.len() > 10 {
            panic_with_error!(&env, Error::TooManyTokens);
        }
        for i in 0..accepted_tokens.len() {
            let t_i = accepted_tokens.get(i).unwrap();
            if accepted_tokens.last_index_of(&t_i) != Some(i) {
                panic_with_error!(&env, Error::DuplicateToken);
            }
        }
        if goal <= 0 || goal > 1_000_000_000_000_000_000_000_000_000_000i128 {
            panic_with_error!(&env, Error::InvalidGoal);
        }
        let now = env.ledger().timestamp();
        if metadata_uri.is_empty() || metadata_uri.len() > MAX_METADATA_URI_LEN {
            panic_with_error!(&env, Error::MetadataCidInvalid);
        }
        let max_deadline = now + 157_680_000;
        if deadline <= now || deadline > max_deadline {
            panic_with_error!(&env, Error::InvalidDeadline);
        }

        // Validate oracle config: if oracles are provided, threshold must be sane.
        let oracle_count = authorized_oracles.len();
        if oracle_count > 0 {
            if oracle_count > MAX_ORACLES || threshold == 0 || threshold > oracle_count {
                panic_with_error!(&env, Error::InvalidOracleConfig);
            }
        }

        let id = get_and_increment_project_id(&env);
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
        };

        save_project(&env, &project);

        if let Some(token) = accepted_tokens.get(0) {
            events::emit_project_created(&env, id, creator, token, goal);
        }

        project
    }

    /// Pause a project, blocking deposits and proof verification/releases.
    ///
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    pub fn pause_project(env: Env, caller: Address, project_id: u64) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);

        let (_config, mut state) = load_project_pair(&env, project_id);
        if !state.paused {
            state.paused = true;
            save_project_state(&env, project_id, &state);
            events::emit_project_paused(&env, project_id, caller);
        }
    }

    /// Unpause a project.
    ///
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    pub fn unpause_project(env: Env, caller: Address, project_id: u64) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);

        let (_config, mut state) = load_project_pair(&env, project_id);
        if state.paused {
            state.paused = false;
            save_project_state(&env, project_id, &state);
            events::emit_project_unpaused(&env, project_id, caller);
        }
    }

    /// Extend a project's deadline.
    ///
    /// - `caller` must hold `ProjectManager`, `Admin`, or `SuperAdmin`.
    /// - Project must be in `Funding` or `Active` state.
    /// - New deadline must be later than the current one.
    /// - Total extension cannot exceed 1 year from the current ledger time.
    pub fn extend_deadline(env: Env, caller: Address, project_id: u64, new_deadline: u64) {
        Self::require_not_paused(&env);
        caller.require_auth();
        rbac::require_can_register(&env, &caller);

        let (mut config, state) = load_project_pair(&env, project_id);

        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            _ => panic_with_error!(&env, Error::ProjectNotActive),
        }

        let now = env.ledger().timestamp();

        // Ensure the project hasn't already expired by current time.
        if now >= config.deadline {
            panic_with_error!(&env, Error::ProjectExpired);
        }
        if new_deadline <= config.deadline {
            panic_with_error!(&env, Error::InvalidDeadline);
        }
        let one_year_from_now = now + 31_536_000;
        if new_deadline > one_year_from_now {
            panic_with_error!(&env, Error::DeadlineTooLong);
        }

        let old_deadline = config.deadline;
        config.deadline = new_deadline;
        save_project_config(&env, project_id, &config);
        events::emit_deadline_extended(&env, project_id, old_deadline, new_deadline);
    }

    pub fn add_to_whitelist(env: Env, caller: Address, project_id: u64, address: Address) {
        caller.require_auth();
        let config = storage::load_project_config(&env, project_id);

        // Auth check: creator or Admin/SuperAdmin
        if caller != config.creator {
            rbac::require_admin_or_above(&env, &caller);
        }
        add_to_whitelist(&env, project_id, &address);
        events::emit_whitelist_added(&env, project_id, address);
    }

    pub fn remove_from_whitelist(env: Env, caller: Address, project_id: u64, address: Address) {
        caller.require_auth();
        let config = storage::load_project_config(&env, project_id);

        // Auth check: creator or Admin/SuperAdmin
        if caller != config.creator {
            rbac::require_admin_or_above(&env, &caller);
        }
        remove_from_whitelist(&env, project_id, &address);
        events::emit_whitelist_removed(&env, project_id, address);
    }

    pub fn get_project(env: Env, id: u64) -> Project {
        load_project(&env, id)
    }

    pub fn get_project_metadata(env: Env, project_id: u64) -> Bytes {
        let config = storage::load_project_config(&env, project_id);
        config.metadata_uri
    }

    pub fn get_balance(env: Env, project_id: u64, token: Address) -> i128 {
        storage::get_token_balance(&env, project_id, &token)
    }

    pub fn get_project_balances(env: Env, project_id: u64) -> ProjectBalances {
        let project = match maybe_load_project(&env, project_id) {
            Some(p) => p,
            None => panic_with_error!(&env, Error::ProjectNotFound),
        };
        get_all_balances(&env, &project)
    }

    pub fn deposit(env: Env, project_id: u64, donator: Address, token: Address, amount: i128) {
        Self::require_not_paused(&env);
        donator.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

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

        // Whitelist check
        if config.is_private && !is_whitelisted(&env, project_id, &donator) {
            panic_with_error!(&env, Error::NotWhitelisted);
        }

        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Expired => panic_with_error!(&env, Error::ProjectExpired),
            _ => panic_with_error!(&env, Error::ProjectNotActive),
        }

        let mut found = false;
        for t in config.accepted_tokens.iter() {
            if t == token {
                found = true;
                break;
            }
        }
        if !found {
            panic_with_error!(&env, Error::TokenNotAccepted);
        }

        let current_donor_balance =
            storage::get_donator_balance(&env, project_id, &token, &donator);
        let is_new_donor = current_donor_balance == 0;

        if is_new_donor {
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

        let new_donor_balance = current_donor_balance
            .checked_add(amount)
            .expect("donator balance overflow");
        storage::set_donator_balance(&env, project_id, &token, &donator, new_donor_balance);

        events::emit_project_funded(&env, project_id, donator, amount);
    }

    pub fn cancel_project(env: Env, caller: Address, project_id: u64) {
        caller.require_auth();
        rbac::require_can_cancel_project(&env, &caller);

        let (config, mut state) = load_project_pair(&env, project_id);
        Self::require_project_not_paused(&env, &state);

        if env.ledger().timestamp() >= config.deadline
            && matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active)
        {
            state.status = ProjectStatus::Expired;
            state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
            save_project_state(&env, project_id, &state);
            panic_with_error!(&env, Error::ProjectExpired);
        }

        if matches!(rbac::get_role(&env, &caller), Some(Role::ProjectManager))
            && caller != config.creator
        {
            panic_with_error!(&env, Error::NotAuthorized);
        }

        if state.status != ProjectStatus::Active {
            panic_with_error!(&env, Error::InvalidTransition);
        }

        state.status = ProjectStatus::Cancelled;
        state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
        save_project_state(&env, project_id, &state);
        events::emit_project_cancelled(&env, project_id, caller);
    }

    pub fn refund(env: Env, donator: Address, project_id: u64, token: Address) {
        donator.require_auth();

        let (config, mut state) = load_project_pair(&env, project_id);

        if env.ledger().timestamp() >= config.deadline
            && matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active)
        {
            state.status = ProjectStatus::Expired;
            state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
            save_project_state(&env, project_id, &state);
        }

        if !matches!(state.status, ProjectStatus::Expired | ProjectStatus::Cancelled) {
            panic_with_error!(&env, Error::ProjectNotExpired);
        }

        if state.refund_expiry > 0 && env.ledger().timestamp() >= state.refund_expiry {
            panic_with_error!(&env, Error::RefundWindowExpired);
        }

        let refund_amount = storage::get_donator_balance(&env, project_id, &token, &donator);
        if refund_amount <= 0 {
            panic_with_error!(&env, Error::InsufficientBalance);
        }

        storage::set_donator_balance(&env, project_id, &token, &donator, 0);
        storage::add_to_token_balance(&env, project_id, &token, -refund_amount);

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&contract_address, &donator, &refund_amount);

        events::emit_refunded(&env, project_id, donator, refund_amount);
    }

    /// Grant the Oracle role globally (legacy single-oracle path).
    pub fn set_oracle(env: Env, caller: Address, oracle: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        rbac::grant_role(&env, &caller, &oracle, Role::Oracle);
    }

    pub fn update_protocol_config(env: Env, caller: Address, fee_recipient: Address, fee_bps: u32) {
        caller.require_auth();
        rbac::require_role(&env, &caller, &Role::SuperAdmin);

        if fee_bps > 1000 {
            panic_with_error!(&env, Error::InvalidFeeBasisPoints);
        }

        let old_config = get_protocol_config(&env);
        let new_config = ProtocolConfig { fee_recipient, fee_bps };
        set_protocol_config(&env, &new_config);
        events::emit_protocol_config_updated(&env, old_config, new_config);
    }

    /// Verify proof of impact and accumulate oracle votes using a BitSet.
    ///
    /// Each authorized oracle calls this once. When `voter_count >= threshold`
    /// the funds are released and the `OracleAgreement` is cleared.
    ///
    /// # BitSet mechanics
    /// - Oracle at index `i` sets bit `i`: `votes |= 1 << i`
    /// - Duplicate detection: if bit `i` is already set, `voter_count` is NOT incremented.
    /// - On threshold: payout fires, agreement storage is cleared.
    pub fn verify_and_release(
        env: Env,
        oracle: Address,
        project_id: u64,
        submitted_proof_hash: BytesN<32>,
    ) {
        Self::require_not_paused(&env);
        oracle.require_auth();

        let (config, mut state) = load_project_pair(&env, project_id);
        Self::require_project_not_paused(&env, &state);

        // Expiry check.
        if env.ledger().timestamp() >= config.deadline
            && matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active)
        {
            state.status = ProjectStatus::Expired;
            state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
            save_project_state(&env, project_id, &state);
            panic_with_error!(&env, Error::ProjectExpired);
        }

        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Completed => panic_with_error!(&env, Error::ThresholdAlreadyMet),
            ProjectStatus::Expired => panic_with_error!(&env, Error::ProjectExpired),
            ProjectStatus::Cancelled => panic_with_error!(&env, Error::InvalidTransition),
        }

        // Proof hash check.
        if submitted_proof_hash != config.proof_hash {
            panic_with_error!(&env, Error::VerificationFailed);
        }

        // ── M-of-N path ──────────────────────────────────────────────────────
        // If the project has an authorized oracle list, use BitSet tracking.
        // Otherwise fall back to the legacy single-oracle (RBAC Oracle role) path.
        if !config.authorized_oracles.is_empty() {
            // Find the calling oracle's index in the authorized list.
            let mut oracle_index: Option<u32> = None;
            for (i, authorized) in config.authorized_oracles.iter().enumerate() {
                if authorized == oracle {
                    oracle_index = Some(i as u32);
                    break;
                }
            }

            let oracle_index = match oracle_index {
                Some(idx) => idx,
                None => panic_with_error!(&env, Error::UnauthorizedOracle),
            };

            // Load (or default-initialize) the in-flight agreement.
            let mut agreement = load_oracle_agreement(&env, project_id);

            let bit = 1u32 << oracle_index;
            let already_voted = (agreement.votes & bit) != 0;

            // Set the bit unconditionally; only increment count if new vote.
            agreement.votes |= bit;
            if !already_voted {
                agreement.voter_count += 1;
            }

            // Emit per-vote event.
            events::emit_oracle_voted(
                &env,
                project_id,
                oracle.clone(),
                oracle_index,
                agreement.voter_count,
                config.threshold,
            );

            // Check threshold.
            if agreement.voter_count < config.threshold {
                // Not yet — persist updated agreement and return.
                save_oracle_agreement(&env, project_id, &agreement);
                return;
            }

            // Threshold met — clear agreement and fall through to payout.
            clear_oracle_agreement(&env, project_id);
        } else {
            // Legacy path: caller must hold the global Oracle RBAC role.
            rbac::require_oracle(&env, &oracle);
        }

        // ── Payout ───────────────────────────────────────────────────────────
        state.status = ProjectStatus::Completed;

        let contract_address = env.current_contract_address();
        let protocol_config = get_protocol_config(&env);

        for token in config.accepted_tokens.iter() {
            let mut balance = drain_token_balance(&env, project_id, &token);

            if balance > 0 {
                let token_client = token::Client::new(&env, &token);

                if let Some(ref pcfg) = protocol_config {
                    if pcfg.fee_bps > 0 {
                        let fee_amount = balance
                            .checked_mul(pcfg.fee_bps as i128)
                            .unwrap_or(0)
                            .checked_div(10000)
                            .unwrap_or(0);

                        if fee_amount > 0 {
                            token_client.transfer(
                                &contract_address,
                                &pcfg.fee_recipient,
                                &fee_amount,
                            );
                            balance = balance.checked_sub(fee_amount).unwrap_or(balance);
                            events::emit_fee_deducted(
                                &env,
                                project_id,
                                token.clone(),
                                fee_amount,
                                pcfg.fee_recipient.clone(),
                            );
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
        events::emit_project_verified(&env, project_id, oracle, submitted_proof_hash);
    }

    pub fn expire_project(env: Env, project_id: u64) {
        let (config, mut state) = load_project_pair(&env, project_id);

        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            _ => panic_with_error!(&env, Error::InvalidTransition),
        }

        if env.ledger().timestamp() < config.deadline {
            panic_with_error!(&env, Error::ProjectNotExpired);
        }

        state.status = ProjectStatus::Expired;
        state.refund_expiry = env.ledger().timestamp() + REFUND_WINDOW;
        save_project_state(&env, project_id, &state);
        events::emit_project_expired(&env, project_id, config.deadline);
    }

    pub fn reclaim_expired_funds(env: Env, creator: Address, project_id: u64) {
        Self::require_not_paused(&env);
        creator.require_auth();

        let (config, state) = load_project_pair(&env, project_id);

        if creator != config.creator {
            panic_with_error!(&env, Error::NotAuthorized);
        }

        if !matches!(state.status, ProjectStatus::Expired | ProjectStatus::Cancelled) {
            panic_with_error!(&env, Error::InvalidTransition);
        }

        if state.refund_expiry == 0 || env.ledger().timestamp() < state.refund_expiry {
            panic_with_error!(&env, Error::RefundWindowActive);
        }

        let contract_address = env.current_contract_address();
        for token in config.accepted_tokens.iter() {
            let balance = drain_token_balance(&env, project_id, &token);
            if balance > 0 {
                let token_client = token::Client::new(&env, &token);
                token_client.transfer(&contract_address, &config.creator, &balance);
                events::emit_expired_funds_reclaimed(
                    &env,
                    project_id,
                    config.creator.clone(),
                    token,
                    balance,
                );
            }
        }
    }

    // ─────────────────────────────────────────────────────────
    // Internal helpers
    // ─────────────────────────────────────────────────────────

    fn require_not_paused(env: &Env) {
        if storage::is_paused(env) {
            panic_with_error!(env, Error::ProtocolPaused);
        }
    }

    fn require_project_not_paused(env: &Env, state: &ProjectState) {
        if state.paused {
            panic_with_error!(env, Error::ProjectPaused);
        }
    }
}
