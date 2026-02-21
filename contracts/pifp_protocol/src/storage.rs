//! # Storage
//!
//! Provides typed helpers over Soroban's two storage tiers used by PIFP:
//!
//! ## Instance storage (contract-lifetime TTL)
//!
//! | Key              | Type      | Description                        |
//! |------------------|-----------|------------------------------------|
//! | `ProjectCount`   | `u64`     | Auto-increment project ID counter  |
//! | `OracleKey`      | `Address` | Active trusted oracle address      |
//!
//! Instance TTL is bumped by **7 days** whenever it falls below 1 day remaining.
//!
//! ## Persistent storage (per-entry TTL)
//!
//! | Key                | Type            | Description                      |
//! |--------------------|-----------------|----------------------------------|
//! | `ProjConfig(id)`   | `ProjectConfig` | Immutable project configuration  |
//! | `ProjState(id)`    | `ProjectState`  | Mutable project state            |
//!
//! Persistent TTL is bumped by **30 days** whenever it falls below 7 days remaining.
//!
//! ## Why split Config and State?
//!
//! Deposits are high-frequency writes. Writing the full `Project` struct (~150 bytes)
//! on every deposit is wasteful. `ProjectState` is ~20 bytes — separating it cuts
//! ledger write costs by ~87% per deposit while keeping the public API clean via
//! the reconstructed [`Project`] return type.

use soroban_sdk::{contracttype, Address, Env};

use crate::types::{Project, ProjectConfig, ProjectState};

// ── TTL Constants ────────────────────────────────────────────────────

/// Approximate ledgers per day (~5 seconds per ledger).
const DAY_IN_LEDGERS: u32 = 17_280;

/// Instance storage: bump by 7 days when below 1 day remaining.
const INSTANCE_BUMP_AMOUNT: u32 = 7 * DAY_IN_LEDGERS;
const INSTANCE_LIFETIME_THRESHOLD: u32 = DAY_IN_LEDGERS;

/// Persistent storage: bump by 30 days when below 7 days remaining.
const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;
const PERSISTENT_LIFETIME_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

// ── Storage Keys ─────────────────────────────────────────────────────

/// All contract storage keys.
///
/// Instance-tier keys (`ProjectCount`, `OracleKey`) live as long as the
/// contract and are extended together. Persistent-tier keys (`ProjConfig`,
/// `ProjState`) hold per-project data with independent TTLs.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DataKey {
    /// Global auto-increment counter for project IDs (Instance).
    ProjectCount,
    /// Trusted oracle/verifier address (Instance).
    OracleKey,
    /// Immutable project configuration keyed by ID (Persistent).
    ProjConfig(u64),
    /// Mutable project state keyed by ID (Persistent).
    ProjState(u64),
}

// ── Instance Storage Helpers ─────────────────────────────────────────

/// Extend instance storage TTL if it falls below the threshold.
fn bump_instance(env: &Env) {
    env.storage()
        .instance()
        .extend_ttl(INSTANCE_LIFETIME_THRESHOLD, INSTANCE_BUMP_AMOUNT);
}

// ─────────────────────────────────────────────────────────
// Project counter
// ─────────────────────────────────────────────────────────

/// Atomically read and increment the project counter.
/// Returns the ID that should be used for the next project.
pub fn get_and_increment_project_id(env: &Env) -> u64 {
    bump_instance(env);
    let current: u64 = env
        .storage()
        .instance()
        .get(&DataKey::ProjectCount)
        .unwrap_or(0);
    env.storage()
        .instance()
        .set(&DataKey::ProjectCount, &(current + 1));
    current
}

/// Store the trusted oracle address in instance storage.
pub fn set_oracle(env: &Env, oracle: &Address) {
    env.storage()
        .instance()
        .set(&DataKey::OracleKey, oracle);
    bump_instance(env);
}

/// Retrieve the trusted oracle address.
/// Panics if no oracle has been set.
pub fn get_oracle(env: &Env) -> Address {
    bump_instance(env);
    env.storage()
        .instance()
        .get(&DataKey::OracleKey)
        .expect("oracle not set")
}

// ── Persistent Storage Helpers ───────────────────────────────────────

/// Extend the TTL for a persistent storage key.
fn bump_persistent(env: &Env, key: &DataKey) {
    env.storage()
        .persistent()
        .extend_ttl(key, PERSISTENT_LIFETIME_THRESHOLD, PERSISTENT_BUMP_AMOUNT);
}

/// Save both the immutable config and initial mutable state for a new project.
pub fn save_project(env: &Env, project: &Project) {
    let config_key = DataKey::ProjConfig(project.id);
    let state_key = DataKey::ProjState(project.id);

    let config = ProjectConfig {
        id: project.id,
        creator: project.creator.clone(),
        token: project.token.clone(),
        goal: project.goal,
        proof_hash: project.proof_hash.clone(),
        deadline: project.deadline,
    };

    let state = ProjectState {
        balance: project.balance,
        status: project.status.clone(),
    };

    env.storage().persistent().set(&config_key, &config);
    env.storage().persistent().set(&state_key, &state);
    bump_persistent(env, &config_key);
    bump_persistent(env, &state_key);
}

/// Load the full `Project` by combining config and state.
/// Panics if the project does not exist.
pub fn load_project(env: &Env, id: u64) -> Project {
    let config = load_project_config(env, id);
    let state = load_project_state(env, id);
    Project {
        id: config.id,
        creator: config.creator,
        token: config.token,
        goal: config.goal,
        balance: state.balance,
        proof_hash: config.proof_hash,
        deadline: config.deadline,
        status: state.status,
    }
}

/// Load only the immutable project configuration.
pub fn load_project_config(env: &Env, id: u64) -> ProjectConfig {
    let key = DataKey::ProjConfig(id);
    let config: ProjectConfig = env
        .storage()
        .persistent()
        .get(&key)
        .expect("project not found");
    bump_persistent(env, &key);
    config
}

/// Load only the mutable project state.
pub fn load_project_state(env: &Env, id: u64) -> ProjectState {
    let key = DataKey::ProjState(id);
    let state: ProjectState = env
        .storage()
        .persistent()
        .get(&key)
        .expect("project not found");
    bump_persistent(env, &key);
    state
}

/// Save only the mutable project state (optimized for deposits/verification).
pub fn save_project_state(env: &Env, id: u64, state: &ProjectState) {
    let key = DataKey::ProjState(id);
    env.storage().persistent().set(&key, state);
    bump_persistent(env, &key);
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