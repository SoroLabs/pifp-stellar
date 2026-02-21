// contracts/pifp_protocol/src/storage.rs
//
// Storage helpers for PifpProtocol.
//
// Multi-asset additions:
//   - DataKey::TokenBalance(project_id, token) → i128
//       Stores the balance of a specific token held for a project.
//       Written on every deposit, read during release.
//   - get_token_balance / set_token_balance / add_to_token_balance
//   - get_all_balances — iterates accepted_tokens and reads each balance

use soroban_sdk::{panic_with_error, Address, Env, Vec};

use crate::{
    types::{Project, ProjectBalances, TokenBalance},
    DataKey, Error,
};

// ─────────────────────────────────────────────────────────
// Project counter
// ─────────────────────────────────────────────────────────

/// Atomically read and increment the project counter.
/// Returns the ID that should be used for the next project.
pub fn get_and_increment_project_id(env: &Env) -> u64 {
    let id: u64 = env
        .storage()
        .persistent()
        .get(&DataKey::ProjectCount)
        .unwrap_or(0);
    env.storage()
        .persistent()
        .set(&DataKey::ProjectCount, &(id + 1));
    id
}

// ─────────────────────────────────────────────────────────
// Project CRUD
// ─────────────────────────────────────────────────────────

/// Persist a project. Overwrites any existing record at the same ID.
pub fn save_project(env: &Env, project: &Project) {
    env.storage()
        .persistent()
        .set(&DataKey::Project(project.id), project);
}

/// Load a project by ID. Panics with `Error::ProjectNotFound` if missing.
pub fn load_project(env: &Env, id: u64) -> Project {
    env.storage()
        .persistent()
        .get(&DataKey::Project(id))
        .unwrap_or_else(|| panic_with_error!(env, Error::ProjectNotFound))
}

// ─────────────────────────────────────────────────────────
// Per-token balance helpers
// ─────────────────────────────────────────────────────────

/// Read the current balance of `token` held for `project_id`.
/// Returns 0 if no deposit has ever been made for this (project, token) pair.
pub fn get_token_balance(env: &Env, project_id: u64, token: &Address) -> i128 {
    env.storage()
        .persistent()
        .get(&DataKey::TokenBalance(project_id, token.clone()))
        .unwrap_or(0i128)
}

/// Overwrite the balance of `token` for `project_id`.
pub fn set_token_balance(env: &Env, project_id: u64, token: &Address, balance: i128) {
    env.storage()
        .persistent()
        .set(&DataKey::TokenBalance(project_id, token.clone()), &balance);
}

/// Add `amount` to the existing balance of `token` for `project_id`.
/// Returns the new balance.
pub fn add_to_token_balance(env: &Env, project_id: u64, token: &Address, amount: i128) -> i128 {
    let current = get_token_balance(env, project_id, token);
    let new_balance = current + amount;
    set_token_balance(env, project_id, token, new_balance);
    new_balance
}

/// Zero out the balance of `token` for `project_id` and return what it was.
/// Called during `verify_and_release` after transferring funds to the creator.
pub fn drain_token_balance(env: &Env, project_id: u64, token: &Address) -> i128 {
    let balance = get_token_balance(env, project_id, token);
    if balance > 0 {
        set_token_balance(env, project_id, token, 0);
    }
    balance
}

/// Build a `ProjectBalances` snapshot by reading each accepted token's balance.
pub fn get_all_balances(env: &Env, project: &Project) -> ProjectBalances {
    let mut balances: Vec<TokenBalance> = Vec::new(env);
    for token in project.accepted_tokens.iter() {
        let balance = get_token_balance(env, project.id, &token);
        balances.push_back(TokenBalance { token, balance });
    }
    ProjectBalances {
        project_id: project.id,
        balances,
    }
}