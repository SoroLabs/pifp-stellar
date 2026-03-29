//! # Types
//!
//! Shared data structures used across all modules of the PIFP protocol.
//!
//! ## Design decisions
//!
//! ### Config / State split
//!
//! A `Project` is internally stored as two separate ledger entries:
//!
//! - [`ProjectConfig`] — written once at registration; never mutated.
//! - [`ProjectState`] — written on every deposit and on verification.
//!
//! The public API exposes the reconstructed [`Project`] struct for convenience.
//!
//! ### Status as a Finite-State Machine
//!
//! [`ProjectStatus`] enforces a strict forward-only lifecycle:
//!
//! ```text
//! Funding ──► Active ──► Completed
//!     └──────────────────►┘
//!     └──► Expired
//! Active ──► Expired
//! Active ──► Cancelled
//! ```
//!
//! Backward transitions and transitions out of terminal states (`Completed`,
//! `Expired`, `Cancelled`) are rejected by lifecycle entrypoints.

use soroban_sdk::{contracttype, Address, Bytes, BytesN, Vec};

/// Current lifecycle state of a funding project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectStatus {
    /// Accepting donations, goal not yet reached.
    Funding,
    /// Goal reached; work in progress (oracle has not yet verified).
    Active,
    /// Oracle verified the proof; funds released to creator.
    Completed,
    /// Deadline passed without reaching goal or verification.
    Expired,
    /// Project was manually cancelled after becoming active.
    /// Remaining donor balances stay refundable.
    Cancelled,
}

/// Immutable project configuration, written once at registration.
///
/// Stored separately from mutable state to reduce write costs on deposits
/// and verification (only ~20 bytes for state vs ~150 bytes for the full struct).
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectConfig {
    pub id: u64,
    pub creator: Address,
    pub accepted_tokens: Vec<Address>,
    pub goal: i128,
    pub proof_hash: BytesN<32>,
    pub deadline: u64,
    pub is_private: bool,
    pub metadata_uri: Bytes,
}

/// Mutable project state, updated on deposits and verification.
///
/// Kept small so that frequent writes (deposits) are cheap.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectState {
    pub status: ProjectStatus,
    /// Count of unique (donator, token) pairs that have deposited.
    pub donation_count: u32,
    /// Ledger timestamp after which donors can no longer refund and the
    /// creator may reclaim unclaimed funds.  Set to `deadline + REFUND_WINDOW`
    /// when the project transitions to Expired, or `cancel_time + REFUND_WINDOW`
    /// when cancelled.  Zero while the project is still in a non-terminal state.
    pub refund_expiry: u64,
}

/// Full on-chain representation of a funding project.
///
/// Used as the public API return type; reconstructed internally from
/// the split `ProjectConfig` + `ProjectState` storage entries.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Auto-incremented unique ID.
    pub id: u64,
    /// Address that registered and will receive released funds.
    pub creator: Address,
    /// Ordered list of SAC token addresses this project accepts.
    /// Set once at registration; cannot be changed after creation.
    /// Length: 1–10 tokens.
    pub accepted_tokens: soroban_sdk::Vec<Address>,
    /// Funding goal expressed in the *first* accepted token's units.
    /// Used as a reference denominator; cross-token goals require off-chain logic.
    pub goal: i128,
    /// Content hash (e.g. IPFS CID digest) of proof artifacts.
    pub proof_hash: soroban_sdk::BytesN<32>,
    /// Optional CID or URI pointing to external project metadata.
    pub metadata_uri: soroban_sdk::Bytes,
    /// Ledger timestamp by which the project must be completed.
    pub deadline: u64,
    /// Current lifecycle state.
    pub status: ProjectStatus,
    /// Count of unique (token, donator) pairs that have donated.
    /// Informational; incremented on each new deposit.
    pub donation_count: u32,
    /// Is this a private project (whitelist only)?
    pub is_private: bool,
    /// Ledger timestamp after which donors can no longer refund and the
    /// creator may reclaim unclaimed funds.  Zero while non-terminal.
    pub refund_expiry: u64,
}

impl Project {
    /// Check whether `token` is in this project's accepted list.
    pub fn accepts_token(&self, token: &Address) -> bool {
        for t in self.accepted_tokens.iter() {
            if &t == token {
                return true;
            }
        }
        false
    }
}

/// Snapshot of all balances for a project — returned by `get_balances`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TokenBalance {
    pub token: Address,
    pub balance: i128,
}

/// Full balance view returned by `get_project_balances`.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProjectBalances {
    pub project_id: u64,
    pub balances: Vec<TokenBalance>,
}
/// Global protocol configuration managed by the SuperAdmin.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    /// Recipient of the platform fee.
    pub fee_recipient: Address,
    /// Platform fee in basis points (1 BPS = 0.01%).
    pub fee_bps: u32,
}
