// contracts/pifp_protocol/src/lib.rs
//
// RBAC-integrated PifpProtocol contract.
//
// Changes from the original:
//   1. Added `mod rbac` — the new Role-Based Access Control module.
//   2. `DataKey` gains no new variants (role storage lives in `RbacKey` inside rbac.rs).
//   3. `Error` gains two new variants: `AlreadyInitialized` and `RoleNotFound`.
//   4. New entry point: `init(env, super_admin)` — must be called once after deployment.
//   5. New entry points for role management: `grant_role`, `revoke_role`,
//      `transfer_super_admin`, `role_of`, `has_role`.
//   6. `set_oracle` now calls `rbac::grant_role(..., Role::Oracle)` instead of writing
//      a bare address — the oracle is just an address with the Oracle role.
//   7. `verify_and_release` uses `rbac::require_oracle` instead of the old `get_oracle`.
//   8. `register_project` uses `rbac::require_can_register` — SuperAdmin, Admin, and
//      ProjectManager may register; an unauthenticated address cannot.

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
    Env,
};

mod storage;
mod types;
pub mod rbac;
pub mod events;

// #[cfg(test)]
// mod fuzz_test;
#[cfg(test)]
mod invariants;
// #[cfg(test)]
// mod test;
#[cfg(test)]
mod test_events;

use storage::{
    get_and_increment_project_id, get_oracle, load_project, load_project_config,
    load_project_state, save_project, save_project_state, set_oracle,
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
    // New in RBAC integration:
    AlreadyInitialized    = 8,
    RoleNotFound          = 9,
}

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {
    // ─────────────────────────────────────────────────────────
    // Initialisation (new)
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
    // Role management (new)
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
    // Existing entry points — updated to use RBAC
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
            token: token.clone(),
            goal,
            proof_hash,
            deadline,
            status: ProjectStatus::Funding,
            donation_count: 0,
        };

        save_project(&env, &project);

        // Standardized event emission
        events::emit_project_created(&env, id, creator, token, goal);

        project
    }

    /// Retrieve a project by its ID.
    pub fn get_project(env: Env, id: u64) -> Project {
        load_project(&env, id)
    }

    /// Deposit funds into a project.
    ///
    /// Reads only the immutable config (for the token address) and the mutable
    /// state, then writes back only the small state entry (~20 bytes) instead
    /// of the full project struct (~150 bytes).
    pub fn deposit(env: Env, project_id: u64, donator: Address, amount: i128) {
        donator.require_auth();

        // Read config for token address; read state for balance.
        let config = load_project_config(&env, project_id);
        let mut state = load_project_state(&env, project_id);

        // Transfer tokens from donator to contract.
        let token_client = token::Client::new(&env, &config.token);
        token_client.transfer(&donator, &env.current_contract_address(), &amount);

        // Update only the mutable state.
        state.balance += amount;
        save_project_state(&env, project_id, &state);

        // Standardized event emission
        events::emit_project_funded(&env, project_id, donator.clone(), amount);
    }

    /// Grant the Oracle role to `oracle`.
    ///
    /// Replaces the original `set_oracle(admin, oracle)`.
    /// - `caller` must hold `SuperAdmin` or `Admin`.
    ///
    /// If an address already holds the Oracle role, calling this with a new
    /// address will grant Oracle to the new one; the old one retains its role
    /// unless explicitly revoked. If you want a single oracle, revoke the old
    /// one first, then call `set_oracle`.
    pub fn set_oracle(env: Env, caller: Address, oracle: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        rbac::grant_role(&env, &caller, &oracle, Role::Oracle);
        set_oracle(&env, &oracle);
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
    pub fn verify_and_release(env: Env, project_id: u64, submitted_proof_hash: BytesN<32>) {
        // Ensure caller is the registered oracle.
        let oracle = get_oracle(&env);
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