//! # PIFP Protocol Contract
//!
//! This is the root crate of the **Proof-of-Impact Funding Protocol (PIFP)**.
//! It exposes the single Soroban contract `PifpProtocol` whose entry points cover
//! the full project lifecycle:
//!
//! | Phase        | Entry Point(s)                              |
//! |--------------|---------------------------------------------|
//! | Bootstrap    | [`PifpProtocol::init`]                      |
//! | Role admin   | `grant_role`, `revoke_role`, `transfer_super_admin`, `set_oracle` |
//! | Registration | [`PifpProtocol::register_project`]          |
//! | Funding      | [`PifpProtocol::deposit`]                   |
//! | Donor safety | [`PifpProtocol::refund`]                    |
//! | Verification | [`PifpProtocol::verify_and_release`]        |
//! | Queries      | `get_project`, `get_project_balances`, `role_of`, `has_role` |
//!
//! ## Architecture
//!
//! Authorization is fully delegated to [`rbac`].  Storage access is fully
//! delegated to [`storage`].  This file contains **only** the public entry
//! points and event emissions — no business logic lives here directly.
//!
//! See [`ARCHITECTURE.md`](../../../../ARCHITECTURE.md) for the full system
//! architecture and threat model.

#![no_std]

use soroban_sdk::{contract, contractimpl, panic_with_error, token, Address, BytesN, Env, Vec};

pub mod errors;
pub mod events;
pub mod invariants_checker;
pub mod rbac;
mod storage;
mod types;

#[cfg(test)]
mod fuzz_test;
#[cfg(test)]
mod rbac_test;

#[cfg(test)]
mod test;
#[cfg(test)]
mod test_donation_count;
#[cfg(test)]
mod test_errors;
#[cfg(test)]
mod test_events;
#[cfg(test)]
mod test_expire;
#[cfg(test)]
mod test_refund;
#[cfg(test)]
mod test_utils;

pub use errors::Error;
pub use events::emit_funds_released;
pub use rbac::Role;
use storage::{
    drain_token_balance, get_all_balances, get_and_increment_project_id, load_project,
    load_project_pair, maybe_load_project, save_project, save_project_state,
};
pub use types::{Project, ProjectBalances, ProjectStatus};

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {
    // ─────────────────────────────────────────────────────────
    // Initialisation
    // ─────────────────────────────────────────────────────────

    /// Initialise the contract and set the first SuperAdmin.
    ///
    /// Must be called exactly once immediately after deployment.
    /// Subsequent calls panic with `Error::AlreadyInitialized`.
    ///
    /// - `super_admin` is granted the `SuperAdmin` role and must sign the transaction.
    pub fn init(env: Env, super_admin: Address) {
        super_admin.require_auth();
        rbac::init_super_admin(&env, &super_admin);
    }

    // ─────────────────────────────────────────────────────────
    // Role management
    // ─────────────────────────────────────────────────────────

    /// Grant `role` to `target`.
    ///
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    /// - Only `SuperAdmin` can grant `SuperAdmin`.
    pub fn grant_role(env: Env, caller: Address, target: Address, role: Role) {
        rbac::grant_role(&env, &caller, &target, role);
    }

    /// Revoke any role from `target`.
    ///
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    /// - Cannot be used to remove the SuperAdmin; use `transfer_super_admin`.
    pub fn revoke_role(env: Env, caller: Address, target: Address) {
        rbac::revoke_role(&env, &caller, &target);
    }

    /// Transfer SuperAdmin to `new_super_admin`.
    ///
    /// - `current_super_admin` must authorize and hold the `SuperAdmin` role.
    /// - The previous SuperAdmin loses the role immediately.
    pub fn transfer_super_admin(env: Env, current_super_admin: Address, new_super_admin: Address) {
        rbac::transfer_super_admin(&env, &current_super_admin, &new_super_admin);
    }

    /// Return the role held by `address`, or `None`.
    pub fn role_of(env: Env, address: Address) -> Option<Role> {
        rbac::role_of(&env, address)
    }

    /// Return `true` if `address` holds `role`.
    pub fn has_role(env: Env, address: Address, role: Role) -> bool {
        rbac::has_role(&env, address, role)
    }

    // ─────────────────────────────────────────────────────────
    // Emergency Control
    // ─────────────────────────────────────────────────────────

    /// Pause the protocol, halting all registrations, deposits, and releases.
    ///
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    pub fn pause(env: Env, caller: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        storage::set_paused(&env, true);
        events::emit_protocol_paused(&env, caller);
    }

    /// Unpause the protocol.
    ///
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    pub fn unpause(env: Env, caller: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        storage::set_paused(&env, false);
        events::emit_protocol_unpaused(&env, caller);
    }

    /// Return true if the protocol is paused.
    pub fn is_paused(env: Env) -> bool {
        storage::is_paused(&env)
    }

    // ─────────────────────────────────────────────────────────
    // Project lifecycle
    // ─────────────────────────────────────────────────────────

    /// Register a new funding project.
    ///
    /// `creator` must hold the `ProjectManager`, `Admin`, or `SuperAdmin` role.
    pub fn register_project(
        env: Env,
        creator: Address,
        accepted_tokens: Vec<Address>,
        goal: i128,
        proof_hash: BytesN<32>,
        deadline: u64,
    ) -> Project {
        Self::require_not_paused(&env);
        creator.require_auth();
        // RBAC gate: only authorised roles may create projects.
        rbac::require_can_register(&env, &creator);

        if accepted_tokens.is_empty() {
            panic_with_error!(&env, Error::EmptyAcceptedTokens);
        }
        if accepted_tokens.len() > 10 {
            panic_with_error!(&env, Error::TooManyTokens);
        }

        // Check for duplicate tokens
        for i in 0..accepted_tokens.len() {
            let t_i = accepted_tokens.get(i).unwrap();
            if accepted_tokens.last_index_of(&t_i) != Some(i) {
                panic_with_error!(&env, Error::DuplicateToken);
            }
        }

        if goal <= 0 || goal > 1_000_000_000_000_000_000_000_000_000_000i128 {
            // 10^30
            panic_with_error!(&env, Error::InvalidGoal);
        }

        let now = env.ledger().timestamp();
        // Max 5 years deadline (5 * 365 * 24 * 60 * 60)
        let max_deadline = now + 157_680_000;
        if deadline <= now || deadline > max_deadline {
            panic_with_error!(&env, Error::InvalidDeadline);
        }

        let id = get_and_increment_project_id(&env);
        let project = Project {
            id,
            creator: creator.clone(),
            accepted_tokens: accepted_tokens.clone(),
            goal,
            proof_hash,
            deadline,
            status: ProjectStatus::Funding,
            donation_count: 0,
        };

        save_project(&env, &project);

        // Standardized event emission
        if let Some(token) = accepted_tokens.get(0) {
            events::emit_project_created(&env, id, creator, token, goal);
        }

        project
    }

    pub fn get_project(env: Env, id: u64) -> Project {
        load_project(&env, id)
    }

    /// Return the balance of `token` for `project_id`.
    pub fn get_balance(env: Env, project_id: u64, token: Address) -> i128 {
        storage::get_token_balance(&env, project_id, &token)
    }

    /// Return the current per-token balances for a project.
    ///
    /// Reconstructs the balance snapshot from persistent storage for every
    /// token that was accepted at registration time.
    ///
    /// # Errors
    /// Panics with `Error::ProjectNotFound` if `project_id` does not exist.
    pub fn get_project_balances(env: Env, project_id: u64) -> ProjectBalances {
        let project = match maybe_load_project(&env, project_id) {
            Some(p) => p,
            None => panic_with_error!(&env, Error::ProjectNotFound),
        };
        get_all_balances(&env, &project)
    }

    /// Deposit funds into a project.
    ///
    /// The `token` must be one of the project's accepted tokens.
    pub fn deposit(env: Env, project_id: u64, donator: Address, token: Address, amount: i128) {
        Self::require_not_paused(&env);
        donator.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, Error::InvalidAmount);
        }

        // Read both config and state with a single helper that bumps TTLs
        // atomically. This is the optimized retrieval pattern; it also returns
        // the state needed for the subsequent checks.
        let (config, mut state) = load_project_pair(&env, project_id);

        // Check expiration
        if env.ledger().timestamp() >= config.deadline {
            if matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active) {
                state.status = ProjectStatus::Expired;
                save_project_state(&env, project_id, &state);
            }
            panic_with_error!(&env, Error::ProjectExpired);
        }

        // Basic status check: must be Funding or Active.
        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Expired => panic_with_error!(&env, Error::ProjectExpired),
            _ => panic_with_error!(&env, Error::ProjectNotActive),
        }

        // Verify token is accepted.
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

        // Check if this is a new unique (donator, token) pair.
        // A donator balance of 0 implicitly proves they have not donated yet, saving a storage key entirely.
        let current_donor_balance =
            storage::get_donator_balance(&env, project_id, &token, &donator);
        let is_new_donor = current_donor_balance == 0;

        if is_new_donor {
            // Increment donation count
            state.donation_count += 1;
            // Save the updated state.
            save_project_state(&env, project_id, &state);
        }

        // Transfer tokens from donator to contract.
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&donator, env.current_contract_address(), &amount);

        // Update the per-token balance.
        let new_balance = storage::add_to_token_balance(&env, project_id, &token, amount);

        // If this is the primary token and goal is reached, transition from Funding to Active.
        if state.status == ProjectStatus::Funding {
            if let Some(first_token) = config.accepted_tokens.get(0) {
                if token == first_token && new_balance >= config.goal {
                    state.status = ProjectStatus::Active;
                    save_project_state(&env, project_id, &state);
                    events::emit_project_active(&env, project_id);
                }
            }
        }

        // Track per-donator refundable amount for this token.
        let new_donor_balance = current_donor_balance
            .checked_add(amount)
            .expect("donator balance overflow");
        storage::set_donator_balance(&env, project_id, &token, &donator, new_donor_balance);

        // Standardized event emission
        events::emit_project_funded(&env, project_id, donator, amount);
    }

    /// Mark an active project as cancelled.
    ///
    /// - `caller` must be `SuperAdmin` or `ProjectManager`.
    /// - If `caller` is `ProjectManager`, it must be the project's creator.
    /// - Only projects in `Active` status may be cancelled.
    pub fn cancel_project(env: Env, caller: Address, project_id: u64) {
        caller.require_auth();
        rbac::require_can_cancel_project(&env, &caller);

        let (config, mut state) = load_project_pair(&env, project_id);

        if env.ledger().timestamp() >= config.deadline
            && matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active)
        {
            state.status = ProjectStatus::Expired;
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
        save_project_state(&env, project_id, &state);
        events::emit_project_cancelled(&env, project_id, caller);
    }

    /// Refund a donator from a cancelled or expired project that was not verified.
    pub fn refund(env: Env, donator: Address, project_id: u64, token: Address) {
        donator.require_auth();

        let (config, mut state) = load_project_pair(&env, project_id);

        if env.ledger().timestamp() >= config.deadline
            && matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active)
        {
            state.status = ProjectStatus::Expired;
            save_project_state(&env, project_id, &state);
        }

        if !matches!(
            state.status,
            ProjectStatus::Expired | ProjectStatus::Cancelled
        ) {
            panic_with_error!(&env, Error::ProjectNotExpired);
        }

        let refund_amount = storage::get_donator_balance(&env, project_id, &token, &donator);
        if refund_amount <= 0 {
            panic_with_error!(&env, Error::InsufficientBalance);
        }

        // Zero-out first to prevent double-refund/reentrancy patterns.
        storage::set_donator_balance(&env, project_id, &token, &donator, 0);
        storage::add_to_token_balance(&env, project_id, &token, -refund_amount);

        let contract_address = env.current_contract_address();
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&contract_address, &donator, &refund_amount);

        events::emit_refunded(&env, project_id, donator, refund_amount);
    }

    /// Grant the Oracle role to `oracle`.
    ///
    /// Replaces the original `set_oracle(admin, oracle)`.
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    pub fn set_oracle(env: Env, caller: Address, oracle: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        rbac::grant_role(&env, &caller, &oracle, Role::Oracle);
    }

    /// Verify proof of impact and release funds to the creator.
    ///
    /// The registered oracle submits a proof hash. If it matches the project's
    /// stored `proof_hash`, the project status transitions to `Completed`.
    ///
    /// NOTE: This is a mocked verification (hash equality).
    /// The structure is prepared for future ZK-STARK verification.
    ///
    /// Reads the immutable config (for proof_hash) and mutable state (for status),
    /// then writes back only the small state entry.
    pub fn verify_and_release(
        env: Env,
        oracle: Address,
        project_id: u64,
        submitted_proof_hash: BytesN<32>,
    ) {
        Self::require_not_paused(&env);
        oracle.require_auth();
        // RBAC gate: caller must hold the Oracle role.
        rbac::require_oracle(&env, &oracle);

        // Optimised dual-read helper
        let (config, mut state) = load_project_pair(&env, project_id);

        if env.ledger().timestamp() >= config.deadline
            && matches!(state.status, ProjectStatus::Funding | ProjectStatus::Active)
        {
            state.status = ProjectStatus::Expired;
            save_project_state(&env, project_id, &state);
            panic_with_error!(&env, Error::ProjectExpired);
        }

        // Ensure the project is in a verifiable state.
        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Completed => panic_with_error!(&env, Error::MilestoneAlreadyReleased),
            ProjectStatus::Expired => panic_with_error!(&env, Error::ProjectExpired),
            ProjectStatus::Cancelled => panic_with_error!(&env, Error::InvalidTransition),
        }

        // Mocked ZK verification: compare submitted hash to stored hash.
        if submitted_proof_hash != config.proof_hash {
            panic_with_error!(&env, Error::VerificationFailed);
        }

        // Transition to Completed — only write the state entry.
        state.status = ProjectStatus::Completed;

        // Transfer all deposited tokens to the creator.
        // If any transfer fails, panic to revert the entire transaction.
        let contract_address = env.current_contract_address();
        for token in config.accepted_tokens.iter() {
            // Drain the token balance (gets balance and zeros it).
            let balance = drain_token_balance(&env, project_id, &token);

            // Only transfer if there's a non-zero balance.
            if balance > 0 {
                // Create token client and transfer to creator.
                let token_client = token::Client::new(&env, &token);
                token_client.transfer(&contract_address, &config.creator, &balance);

                // Emit funds_released event for this token.
                events::emit_funds_released(&env, project_id, token, balance);
            }
        }

        // Save the updated state (now marked as Completed).
        save_project_state(&env, project_id, &state);

        // Standardized event emission
        events::emit_project_verified(&env, project_id, oracle.clone(), submitted_proof_hash);
    }

    /// Mark a project as expired if its deadline has passed.
    ///
    /// Permissionless: anyone can trigger expiration once the deadline is met.
    /// - Panics if project is not in Funding status.
    /// - Panics if deadline has not passed.
    pub fn expire_project(env: Env, project_id: u64) {
        let (config, mut state) = load_project_pair(&env, project_id);

        // State transition check: only Funding or Active projects can expire.
        // Completed projects cannot be expired.
        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            _ => panic_with_error!(&env, Error::InvalidTransition),
        }

        // Deadline check.
        if env.ledger().timestamp() < config.deadline {
            panic_with_error!(&env, Error::ProjectNotExpired);
        }

        // Update status and save.
        state.status = ProjectStatus::Expired;
        save_project_state(&env, project_id, &state);

        // Standardized event emission.
        events::emit_project_expired(&env, project_id, config.deadline);
    }

    // ─────────────────────────────────────────────────────────
    // Internal Helpers
    // ─────────────────────────────────────────────────────────

    fn require_not_paused(env: &Env) {
        if storage::is_paused(env) {
            panic_with_error!(env, Error::ProtocolPaused);
        }
    }
}
