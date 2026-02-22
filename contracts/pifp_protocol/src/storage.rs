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

use soroban_sdk::{contracttype, Address, Env, Vec};

use crate::types::{Project, ProjectBalances, ProjectConfig, ProjectState, TokenBalance};

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
    /// Immutable project configuration keyed by ID (Persistent).
    ProjConfig(u64),
    /// Mutable project state keyed by ID (Persistent).
    ProjState(u64),
    /// Token balance for a specific project and token (Persistent).
    TokenBalance(u64, Address),
    /// Protocol pause state (Instance).
    IsPaused,
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

/// Return true if the protocol is currently paused.
pub fn is_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::IsPaused)
        .unwrap_or(false)
}

/// Set the protocol's pause state.
pub fn set_paused(env: &Env, paused: bool) {
    bump_instance(env);
    env.storage().instance().set(&DataKey::IsPaused, &paused);
}

// ── Persistent Storage Helpers ───────────────────────────────────────

/// Extend the TTL for a persistent storage key.
fn bump_persistent(env: &Env, key: &DataKey) {
    env.storage().persistent().extend_ttl(
        key,
        PERSISTENT_LIFETIME_THRESHOLD,
        PERSISTENT_BUMP_AMOUNT,
    );
}

/// Save both the immutable config and initial mutable state for a new project.
pub fn save_project(env: &Env, project: &Project) {
    let config_key = DataKey::ProjConfig(project.id);
    let state_key = DataKey::ProjState(project.id);

    let config = ProjectConfig {
        id: project.id,
        creator: project.creator.clone(),
        accepted_tokens: project.accepted_tokens.clone(),
        goal: project.goal,
        proof_hash: project.proof_hash.clone(),
        deadline: project.deadline,
    };

    let state = ProjectState {
        status: project.status.clone(),
        donation_count: project.donation_count,
    };

    env.storage().persistent().set(&config_key, &config);
    env.storage().persistent().set(&state_key, &state);
    bump_persistent(env, &config_key);
    bump_persistent(env, &state_key);

    // Initialise balances to 0 for all accepted tokens.
    for token in project.accepted_tokens.iter() {
        set_token_balance(env, project.id, &token, 0);
    }
}

// /// Load the full `Project` by combining config and state.
// /// Panics if the project does not exist.
// pub fn load_project(env: &Env, id: u64) -> Project {
//     let config = load_project_config(env, id);
//     let state = load_project_state(env, id);
//     Project {
//         id: config.id,
//         creator: config.creator,
//         accepted_tokens: config.accepted_tokens,
//         goal: config.goal,
//         proof_hash: config.proof_hash,
//         deadline: config.deadline,
//         status: state.status,
//         donation_count: 0, // In a real system, this might be tracked in ProjectState
//     }
// }

/// Load only the immutable project configuration.
///
/// This helper panics with a generic string if the project does not exist. It
/// is a thin wrapper around [`maybe_load_project_config`].
#[allow(dead_code)]
pub fn load_project_config(env: &Env, id: u64) -> ProjectConfig {
    maybe_load_project_config(env, id).expect("project not found")
}

/// Load only the mutable project state.
///
/// Panics with a generic string if the project does not exist; delegates to
/// [`maybe_load_project_state`].
#[allow(dead_code)]
pub fn load_project_state(env: &Env, id: u64) -> ProjectState {
    maybe_load_project_state(env, id).expect("project not found")
}

/// Save only the mutable project state (optimized for deposits/verification).
pub fn save_project_state(env: &Env, id: u64, state: &ProjectState) {
    let key = DataKey::ProjState(id);
    env.storage().persistent().set(&key, state);
    bump_persistent(env, &key);
}

// ── New retrieval helpers ─────────────────────────────────────────

/// Returns `true` if a project with the given `id` exists in persistent storage.
///
/// This performs a *single* storage `has` check and does **not** bump the TTL.
/// It can be useful for quick existence guards without expensive panics or
/// unwrapping.
#[allow(dead_code)]
pub fn project_exists(env: &Env, id: u64) -> bool {
    let config_key = DataKey::ProjConfig(id);
    env.storage().persistent().has(&config_key)
}

/// Attempt to load the immutable configuration for `id`.
///
/// The returned option will be `None` if the project is not found. When a value
/// is returned the entry's TTL is bumped as usual; if the project does not
/// exist **no TTL bump occurs**.
#[allow(dead_code)]
pub fn maybe_load_project_config(env: &Env, id: u64) -> Option<ProjectConfig> {
    let key = DataKey::ProjConfig(id);
    let opt: Option<ProjectConfig> = env.storage().persistent().get(&key);
    if opt.is_some() {
        bump_persistent(env, &key);
    }
    opt
}

/// Attempt to load the mutable state for `id`.
///
/// Works analogously to [`maybe_load_project_config`].
#[allow(dead_code)]
pub fn maybe_load_project_state(env: &Env, id: u64) -> Option<ProjectState> {
    let key = DataKey::ProjState(id);
    let opt: Option<ProjectState> = env.storage().persistent().get(&key);
    if opt.is_some() {
        bump_persistent(env, &key);
    }
    opt
}

/// Fetch both config and state in one call.
///
/// This is the core of our “optimized retrieval pattern”. Instead of the
/// caller performing two separate lookups (which would each bump TTLs and
/// incur independent gas costs), this helper reads both entries, bumps both
/// TTLs, and returns them together. It is heavily used by high‑frequency
/// operations such as `deposit` and `verify_and_release`.
///
/// Panics with `project not found` if either component is missing.
pub fn load_project_pair(env: &Env, id: u64) -> (ProjectConfig, ProjectState) {
    let config_key = DataKey::ProjConfig(id);
    let state_key = DataKey::ProjState(id);

    let config: ProjectConfig = env
        .storage()
        .persistent()
        .get(&config_key)
        .expect("project not found");
    let state: ProjectState = env
        .storage()
        .persistent()
        .get(&state_key)
        .expect("project not found");

    bump_persistent(env, &config_key);
    bump_persistent(env, &state_key);

    (config, state)
}

/// Load the full `Project` by combining config and state.
///
/// Internally this now just delegates to [`load_project_pair`], avoiding
/// duplicate TTL bumps and read boilerplate.
pub fn load_project(env: &Env, id: u64) -> Project {
    let (config, state) = load_project_pair(env, id);
    Project {
        id: config.id,
        creator: config.creator,
        accepted_tokens: config.accepted_tokens,
        goal: config.goal,
        proof_hash: config.proof_hash,
        deadline: config.deadline,
        status: state.status,
        donation_count: state.donation_count,
    }
}

/// Attempt to load a full project, returning `None` if it does not exist.
///
/// This is the most efficient way to query the contract when callers are
/// unsure whether the project exists; it avoids any panics and still bumps the
/// TTL of both underlying entries when present.
#[allow(dead_code)]
pub fn maybe_load_project(env: &Env, id: u64) -> Option<Project> {
    let config_key = DataKey::ProjConfig(id);
    // We test existence on one key only; if a project is corrupt (config
    // without state) the subsequent `get` will still panic, which is acceptable
    // since such a situation should never occur in normal operation.
    if !env.storage().persistent().has(&config_key) {
        return None;
    }
    let (config, state) = load_project_pair(env, id);
    Some(Project {
        id: config.id,
        creator: config.creator,
        accepted_tokens: config.accepted_tokens,
        goal: config.goal,
        proof_hash: config.proof_hash,
        deadline: config.deadline,
        status: state.status,
        donation_count: 0,
        donation_count: state.donation_count,
    })
}

/// Retrieve the balance of `token` for `project_id`.
pub fn get_token_balance(env: &Env, project_id: u64, token: &Address) -> i128 {
    let key = DataKey::TokenBalance(project_id, token.clone());
    let balance = env.storage().persistent().get(&key).unwrap_or(0);
    bump_persistent(env, &key);
    balance
}

/// Set the balance of `token` for `project_id`.
pub fn set_token_balance(env: &Env, project_id: u64, token: &Address, balance: i128) {
    let key = DataKey::TokenBalance(project_id, token.clone());
    env.storage().persistent().set(&key, &balance);
    bump_persistent(env, &key);
}

/// Add `amount` to the existing balance of `token` for `project_id`.
/// Returns the new balance.
pub fn add_to_token_balance(env: &Env, project_id: u64, token: &Address, amount: i128) -> i128 {
    let current = get_token_balance(env, project_id, token);
    let new_balance = current.checked_add(amount).expect("balance overflow");
    set_token_balance(env, project_id, token, new_balance);
    new_balance
}

/// Zero out the balance of `token` for `project_id` and return what it was.
/// Called during `verify_and_release` after transferring funds to the creator.
#[allow(dead_code)]
pub fn drain_token_balance(env: &Env, project_id: u64, token: &Address) -> i128 {
    let balance = get_token_balance(env, project_id, token);
    if balance > 0 {
        set_token_balance(env, project_id, token, 0);
    }
    balance
}

/// Build a `ProjectBalances` snapshot by reading each accepted token's balance.
#[allow(dead_code)]
pub fn get_all_balances(env: &Env, project: &Project) -> ProjectBalances {
    let mut balances: Vec<TokenBalance> = Vec::new(env);
    for token in project.accepted_tokens.iter() {
        let balance = get_token_balance(env, project.id, &token);
        balances.push_back(TokenBalance { token: token.clone(), balance });
    }
    ProjectBalances {
        project_id: project.id,
        balances,
    }
}
