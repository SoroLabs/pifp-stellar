// contracts/pifp_protocol/src/lib.rs
//
// Multi-Asset Funding — changes on top of the RBAC-integrated version:
//
//   1. DataKey gains `TokenBalance(u64, Address)` — per-(project, token) balance.
//   2. Error gains `TokenNotAccepted = 10`, `ZeroAmount = 11`,
//      `TooManyTokens = 12`, `TokenAlreadyAccepted = 13`.
//   3. `register_project`: `token: Address` → `accepted_tokens: Vec<Address>` (1–10).
//   4. `deposit`: new `token_address: Address` param; whitelist check; per-token storage.
//   5. `verify_and_release`: iterates all accepted tokens, releases each balance.
//   6. New admin functions: `whitelist_token`, `remove_token`.
//   7. New queries: `get_token_balance`, `get_project_balances`, `is_token_accepted`.

#![no_std]

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error,
    symbol_short, token, Address, BytesN, Env, Symbol, Vec,
};

mod storage;
mod types;
pub mod rbac;

#[cfg(test)]
mod test;

use storage::{
    add_to_token_balance, drain_token_balance, get_all_balances,
    get_and_increment_project_id, get_token_balance as storage_get_token_balance,
    load_project, save_project,
};
pub use types::{Project, ProjectBalances, ProjectStatus, TokenBalance};
pub use rbac::Role;

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    ProjectCount,
    Project(u64),
    /// Per-(project_id, token_address) balance: i128
    TokenBalance(u64, Address),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    ProjectNotFound          = 1,
    MilestoneNotFound        = 2,
    MilestoneAlreadyReleased = 3,
    InsufficientBalance      = 4,
    InvalidMilestones        = 5,
    NotAuthorized            = 6,
    GoalMismatch             = 7,
    AlreadyInitialized       = 8,
    RoleNotFound             = 9,
    TokenNotAccepted         = 10,
    ZeroAmount               = 11,
    TooManyTokens            = 12,
    TokenAlreadyAccepted     = 13,
}

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {

    // ═══════════════════════════════════════════════════
    // Initialisation
    // ═══════════════════════════════════════════════════

    pub fn init(env: Env, super_admin: Address) {
        super_admin.require_auth();
        rbac::init_super_admin(&env, &super_admin);
    }

    // ═══════════════════════════════════════════════════
    // RBAC management (unchanged from RBAC session)
    // ═══════════════════════════════════════════════════

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

    pub fn set_oracle(env: Env, caller: Address, oracle: Address) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);
        rbac::grant_role(&env, &caller, &oracle, Role::Oracle);
    }

    // ═══════════════════════════════════════════════════
    // Project registration (updated)
    // ═══════════════════════════════════════════════════

    /// Register a new multi-asset funding project.
    ///
    /// # Changed from single-token version
    /// `token: Address` is replaced by `accepted_tokens: Vec<Address>` (1–10 SAC addresses).
    /// To migrate: pass `vec![&env, old_token_address]`.
    pub fn register_project(
        env: Env,
        creator: Address,
        accepted_tokens: Vec<Address>,
        goal: i128,
        proof_hash: BytesN<32>,
        deadline: u64,
    ) -> Project {
        creator.require_auth();
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
            creator,
            accepted_tokens,
            goal,
            proof_hash,
            deadline,
            status: ProjectStatus::Funding,
            donation_count: 0,
        };

        save_project(&env, &project);

        env.events().publish(
            (symbol_short!("proj_new"), id),
            project.accepted_tokens.clone(),
        );

        project
    }

    /// Retrieve a project by its ID.
    pub fn get_project(env: Env, id: u64) -> Project {
        load_project(&env, id)
    }

    // ═══════════════════════════════════════════════════
    // Multi-asset deposit
    // ═══════════════════════════════════════════════════

    /// Donate `amount` of `token_address` to a project.
    ///
    /// - `token_address` must be on the project whitelist.
    /// - `amount` must be > 0.
    /// - Uses `token::Client::transfer` (standard SAC interface).
    pub fn deposit(
        env: Env,
        project_id: u64,
        donator: Address,
        token_address: Address,
        amount: i128,
    ) {
        donator.require_auth();

        if amount <= 0 {
            panic_with_error!(&env, Error::ZeroAmount);
        }

        let mut project = load_project(&env, project_id);

        match project.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Completed => panic_with_error!(&env, Error::MilestoneAlreadyReleased),
            ProjectStatus::Expired   => panic_with_error!(&env, Error::ProjectNotFound),
        }

        if !project.accepts_token(&token_address) {
            panic_with_error!(&env, Error::TokenNotAccepted);
        }

        // Pull tokens from donator into the contract via the SAC interface
        let token_client = token::Client::new(&env, &token_address);
        token_client.transfer(&donator, &env.current_contract_address(), &amount);

        let new_balance = add_to_token_balance(&env, project_id, &token_address, amount);

        project.donation_count += 1;
        save_project(&env, &project);

        env.events().publish(
            (Symbol::new(&env, "donation_received"), project_id, token_address),
            (donator, amount, new_balance),
        );
    }

    // ═══════════════════════════════════════════════════
    // Token whitelist management
    // ═══════════════════════════════════════════════════

    /// Add a token to a project's accepted list. Admin/SuperAdmin only.
    pub fn whitelist_token(
        env: Env,
        caller: Address,
        project_id: u64,
        token_address: Address,
    ) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);

        let mut project = load_project(&env, project_id);

        if project.accepted_tokens.len() >= 10 {
            panic_with_error!(&env, Error::TooManyTokens);
        }
        if project.accepts_token(&token_address) {
            panic_with_error!(&env, Error::TokenAlreadyAccepted);
        }

        project.accepted_tokens.push_back(token_address.clone());
        save_project(&env, &project);

        env.events().publish(
            (symbol_short!("tok_add"), project_id),
            token_address,
        );
    }

    /// Remove a token from a project's accepted list. Admin/SuperAdmin only.
    /// At least one token must remain. Drain any balance first.
    pub fn remove_token(
        env: Env,
        caller: Address,
        project_id: u64,
        token_address: Address,
    ) {
        caller.require_auth();
        rbac::require_admin_or_above(&env, &caller);

        let mut project = load_project(&env, project_id);

        if !project.accepts_token(&token_address) {
            panic_with_error!(&env, Error::TokenNotAccepted);
        }
        if project.accepted_tokens.len() <= 1 {
            panic_with_error!(&env, Error::InvalidMilestones);
        }

        let mut new_tokens: Vec<Address> = Vec::new(&env);
        for t in project.accepted_tokens.iter() {
            if t != token_address {
                new_tokens.push_back(t);
            }
        }
        project.accepted_tokens = new_tokens;
        save_project(&env, &project);

        env.events().publish(
            (symbol_short!("tok_del"), project_id),
            token_address,
        );
    }

    // ═══════════════════════════════════════════════════
    // Verification and fund release (multi-asset)
    // ═══════════════════════════════════════════════════

    /// Verify proof and release ALL token balances to the creator.
    /// Iterates every accepted token and transfers non-zero balances.
    pub fn verify_and_release(
        env: Env,
        oracle: Address,
        project_id: u64,
        submitted_proof_hash: BytesN<32>,
    ) {
        oracle.require_auth();
        rbac::require_oracle(&env, &oracle);

        let mut project = load_project(&env, project_id);

        match project.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Completed => panic_with_error!(&env, Error::MilestoneAlreadyReleased),
            ProjectStatus::Expired   => panic_with_error!(&env, Error::ProjectNotFound),
        }

        if submitted_proof_hash != project.proof_hash {
            panic_with_error!(&env, Error::GoalMismatch);
        }

        // Release every accepted token balance to the creator
        for token_address in project.accepted_tokens.iter() {
            let balance = drain_token_balance(&env, project_id, &token_address);
            if balance > 0 {
                token::Client::new(&env, &token_address).transfer(
                    &env.current_contract_address(),
                    &project.creator,
                    &balance,
                );
                env.events().publish(
                    (symbol_short!("released"), project_id, token_address),
                    (project.creator.clone(), balance),
                );
            }
        }

        project.status = ProjectStatus::Completed;
        save_project(&env, &project);

        env.events().publish((symbol_short!("verified"),), project_id);
    }

    // ═══════════════════════════════════════════════════
    // Read-only queries
    // ═══════════════════════════════════════════════════

    /// Balance of one specific token held for a project.
    pub fn get_token_balance(env: Env, project_id: u64, token_address: Address) -> i128 {
        storage_get_token_balance(&env, project_id, &token_address)
    }

    /// All token balances for a project.
    pub fn get_project_balances(env: Env, project_id: u64) -> ProjectBalances {
        let project = load_project(&env, project_id);
        get_all_balances(&env, &project)
    }

    /// Whether a token is on the project's whitelist.
    pub fn is_token_accepted(env: Env, project_id: u64, token_address: Address) -> bool {
        let project = load_project(&env, project_id);
        project.accepts_token(&token_address)
    }
}