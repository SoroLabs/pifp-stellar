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
//! | Verification | [`PifpProtocol::verify_and_release`]        |
//! | Queries      | `get_project`, `role_of`, `has_role`        |
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

use soroban_sdk::{
    contract, contracterror, contractimpl, panic_with_error, token, Address, BytesN,
    Env, Vec,
};

mod storage;
mod types;
pub mod rbac;
pub mod events;

#[cfg(test)]
mod invariants;
#[cfg(test)]
mod test;
#[cfg(test)]
mod rbac_test;
#[cfg(test)]
mod fuzz_test;
#[cfg(test)]
mod test_events;

use storage::{
    get_and_increment_project_id, load_project, load_project_config,
    load_project_state, save_project, save_project_state,
};
pub use types::{Project, ProjectStatus};
pub use rbac::Role;

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    ProjectNotFound       = 1,
    MilestoneNotFound     = 2,
    MilestoneAlreadyReleased = 3,
    InsufficientBalance   = 4,
    InvalidMilestones     = 5,
    NotAuthorized         = 6,
    GoalMismatch          = 7,
    AlreadyInitialized    = 8,
    RoleNotFound          = 9,
    TooManyTokens         = 10,
}

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
        creator.require_auth();
        // RBAC gate: only authorised roles may create projects.
        rbac::require_can_register(&env, &creator);

        if accepted_tokens.len() == 0 {
            panic_with_error!(&env, Error::InvalidMilestones);
        }
        if accepted_tokens.len() > 10 {
            panic_with_error!(&env, Error::TooManyTokens);
        }
        if goal <= 0 {
            panic_with_error!(&env, Error::InvalidMilestones);
        }
        if deadline <= env.ledger().timestamp() {
            panic_with_error!(&env, Error::InvalidMilestones);
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

    /// Return a snapshot of all balances for `project_id`.
    pub fn get_balances(env: Env, project_id: u64) -> types::ProjectBalances {
        let project = load_project(&env, project_id);
        storage::get_all_balances(&env, &project)
    }

    /// Deposit funds into a project.
    ///
    /// The `token` must be one of the project's accepted tokens.
    pub fn deposit(env: Env, project_id: u64, donator: Address, token: Address, amount: i128) {
        donator.require_auth();

        // Read config to verify token; read state for status check.
        let config = load_project_config(&env, project_id);
        let state = load_project_state(&env, project_id);

        // Basic status check: must be Funding or Active.
        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            _ => panic!("project not accepting deposits"),
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
            panic!("token not accepted by this project");
        }

        // Transfer tokens from donator to contract.
        let token_client = token::Client::new(&env, &token);
        token_client.transfer(&donator, &env.current_contract_address(), &amount);

        // Update the per-token balance.
        storage::add_to_token_balance(&env, project_id, &token, amount);

        // Standardized event emission
        events::emit_project_funded(&env, project_id, donator, amount);
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
    pub fn verify_and_release(env: Env, oracle: Address, project_id: u64, submitted_proof_hash: BytesN<32>) {
        oracle.require_auth();
        // RBAC gate: caller must hold the Oracle role.
        rbac::require_oracle(&env, &oracle);

        // Read immutable config for proof hash, mutable state for status.
        let config = load_project_config(&env, project_id);
        let mut state = load_project_state(&env, project_id);

        // Ensure the project is in a verifiable state.
        match state.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Completed => panic_with_error!(&env, Error::MilestoneAlreadyReleased),
            ProjectStatus::Expired   => panic_with_error!(&env, Error::ProjectNotFound),
        }

        // Mocked ZK verification: compare submitted hash to stored hash.
        if submitted_proof_hash != config.proof_hash {
            panic!("proof verification failed: hash mismatch");
        }

        // Transition to Completed — only write the state entry.
        state.status = ProjectStatus::Completed;
        save_project_state(&env, project_id, &state);

        // Standardized event emission
        events::emit_project_verified(&env, project_id, oracle.clone(), submitted_proof_hash);
    }
}