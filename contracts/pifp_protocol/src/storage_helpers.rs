//! # Storage Helpers — TTL Bump Trait
//!
//! Provides a standardized [`BumpTtl`] trait for automatic TTL management of
//! persistent Soroban storage entries. Any type that represents a persistent
//! storage value can implement this trait to self-describe its bump thresholds
//! and trigger the correct `extend_ttl` call in one place.
//!
//! ## Why a trait?
//!
//! `storage.rs` already contains a private `bump_persistent(env, key)` helper,
//! but it is key-level: callers must know which key to pass. The [`BumpTtl`]
//! trait lifts this to the *value* level — each type knows its own key pattern
//! and thresholds, so call-sites become a single `.bump_ttl(env, id)` call
//! with no magic numbers scattered around.
//!
//! ## Configurable thresholds
//!
//! All threshold and bump-amount constants are re-exported from this module so
//! that `storage.rs` (and tests) can share a single source of truth.
//!
//! | Constant                         | Value        | Meaning                                       |
//! |----------------------------------|--------------|-----------------------------------------------|
//! | [`DAY_IN_LEDGERS`]               | 17 280       | ~5 s/ledger × 86 400 s/day                    |
//! | [`PERSISTENT_LIFETIME_THRESHOLD`]| 7 × 17 280   | Bump when fewer than 7 days remain            |
//! | [`PERSISTENT_BUMP_AMOUNT`]       | 30 × 17 280  | Extend by 30 days on each bump                |

use soroban_sdk::Env;

use crate::storage::DataKey;

// ── TTL Constants  ───────────────────────────────────

/// Approximate number of ledgers per day at ~5 seconds per ledger.
pub const DAY_IN_LEDGERS: u32 = 17_280;

/// Bump persistent entries by this many ledgers (~30 days).
pub const PERSISTENT_BUMP_AMOUNT: u32 = 30 * DAY_IN_LEDGERS;

/// Trigger a bump when fewer than this many ledgers remain (~7 days).
pub const PERSISTENT_LIFETIME_THRESHOLD: u32 = 7 * DAY_IN_LEDGERS;

// ── BumpTtl Trait ─────────────────────────────────────────────────────────────

/// Implemented by any type whose persistent storage entries should have their
/// TTL automatically extended.
///
/// # Contract
///
/// Implementors must describe:
/// - Which storage key(s) are associated with their `id`.
/// - What threshold and bump amount to use (defaulting to the module constants).
///
/// # Example
///
/// ```rust
/// // Inside an entry point or storage helper:
/// ProjectConfig::bump_ttl(&env, project_id);
/// ProjectState::bump_ttl(&env, project_id);
/// ```
pub trait BumpTtl {
    /// Extend the TTL for the persistent storage entry associated with `id`.
    ///
    /// Implementors should call `env.storage().persistent().extend_ttl(...)` for
    /// every key that belongs to this type.
    fn bump_ttl(env: &Env, id: u64);

    /// The minimum remaining ledgers before a bump is triggered.
    ///
    /// Defaults to [`PERSISTENT_LIFETIME_THRESHOLD`]; override to customise.
    fn lifetime_threshold() -> u32 {
        PERSISTENT_LIFETIME_THRESHOLD
    }

    /// The number of ledgers to extend TTL by when a bump is triggered.
    ///
    /// Defaults to [`PERSISTENT_BUMP_AMOUNT`]; override to customise.
    fn bump_amount() -> u32 {
        PERSISTENT_BUMP_AMOUNT
    }
}

// ── Implementations ───────────────────────────────────────────────────────────

/// TTL management for the immutable project configuration entry (`ProjConfig`).
///
/// A single `extend_ttl` call on the `ProjConfig(id)` key is issued.
/// Threshold: 7 days. Bump: 30 days.
pub struct ProjectConfigTtl;

impl BumpTtl for ProjectConfigTtl {
    fn bump_ttl(env: &Env, id: u64) {
        let key = DataKey::ProjConfig(id);
        env.storage().persistent().extend_ttl(
            &key,
            Self::lifetime_threshold(),
            Self::bump_amount(),
        );
    }
}

/// TTL management for the mutable project state entry (`ProjState`).
///
/// A single `extend_ttl` call on the `ProjState(id)` key is issued.
/// Threshold: 7 days. Bump: 30 days.
pub struct ProjectStateTtl;

impl BumpTtl for ProjectStateTtl {
    fn bump_ttl(env: &Env, id: u64) {
        let key = DataKey::ProjState(id);
        env.storage().persistent().extend_ttl(
            &key,
            Self::lifetime_threshold(),
            Self::bump_amount(),
        );
    }
}

/// TTL management for *both* project entries together — config and state.
///
/// Use this in high-frequency paths (e.g. `load_project_pair`) to extend both
/// keys in two consecutive calls without repeating the threshold/amount values.
///
/// This is the recommended helper for `load_project`, `load_project_pair`, and
/// any path that reads the full project.
pub struct ProjectTtl;

impl BumpTtl for ProjectTtl {
    fn bump_ttl(env: &Env, id: u64) {
        ProjectConfigTtl::bump_ttl(env, id);
        ProjectStateTtl::bump_ttl(env, id);
    }
}

// ── Standalone bump helpers ───────────────────────────────────────────────────
// These free functions wrap the trait implementations for call-sites that
// prefer an imperative style over the trait syntax.

/// Bump the TTL for the `ProjConfig(id)` persistent entry.
///
/// Equivalent to `ProjectConfigTtl::bump_ttl(env, id)`.
#[inline]
pub fn bump_project_config(env: &Env, id: u64) {
    ProjectConfigTtl::bump_ttl(env, id);
}

/// Bump the TTL for the `ProjState(id)` persistent entry.
///
/// Equivalent to `ProjectStateTtl::bump_ttl(env, id)`.
#[inline]
pub fn bump_project_state(env: &Env, id: u64) {
    ProjectStateTtl::bump_ttl(env, id);
}

/// Bump the TTL for both `ProjConfig(id)` and `ProjState(id)` in one call.
///
/// Equivalent to `ProjectTtl::bump_ttl(env, id)`.
#[inline]
pub fn bump_project(env: &Env, id: u64) {
    ProjectTtl::bump_ttl(env, id);
}