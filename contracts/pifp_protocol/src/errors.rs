//! # PIFP Protocol — Centralized Error Registry
//!
//! All contract error codes live here.  A single source-of-truth makes it easy
//! to:
//! - avoid numeric collisions between modules,
//! - generate client-side error messages from one place, and
//! - document the expected failure conditions for each operation.
//!
//! ## Numbering convention
//! Codes are grouped by domain so that new variants can be inserted without
//! renumbering existing ones.  The grouping is:
//!
//! | Range   | Domain                      |
//! |---------|-----------------------------| 
//! | 1 – 9   | Protocol / initialisation   |
//! | 10 – 19 | Project lifecycle           |
//! | 20 – 29 | Funding & release           |
//! | 30 – 39 | Metadata                    |
//! | 40 – 49 | Access control              |

use soroban_sdk::contracterror;

/// Unified error enum for the entire PIFP protocol contract suite.
///
/// Every `panic_with_error!` call in the contract must use a variant from this
/// enum so that callers and indexers can handle failures deterministically.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u32)]
pub enum ContractError {
    // -----------------------------------------------------------------------
    // 1 – 9  Protocol / initialisation errors
    // -----------------------------------------------------------------------

    /// `initialize()` was called more than once.
    ///
    /// The protocol config is write-once; call `set_fee_bps` or `set_treasury`
    /// to update individual fields after deployment.
    ProtocolAlreadyInitialized = 1,

    /// A method that requires the protocol to be initialised was called before
    /// `initialize()` had been executed on this contract instance.
    ProtocolNotInitialized = 2,

    // -----------------------------------------------------------------------
    // 10 – 19  Project lifecycle errors
    // -----------------------------------------------------------------------

    /// The provided `project_id` does not map to any registered project in
    /// instance storage.
    ///
    /// Ensure the correct contract instance and project ID are being used.
    ProjectNotFound = 10,

    /// The `goal` amount supplied to `create_project` was zero or negative.
    ///
    /// A project goal must be a strictly positive integer representing the
    /// minimum amount (in the smallest token unit) required to fund it.
    ProjectGoalNotPositive = 11,

    // -----------------------------------------------------------------------
    // 20 – 29  Funding & release errors
    // -----------------------------------------------------------------------

    /// The `amount` passed to `release` was zero or negative.
    ///
    /// Releases must move a strictly positive amount of funds.
    ReleaseAmountIsZero = 20,

    /// The requested release amount exceeds the project's current on-chain
    /// balance.
    ///
    /// Deposit more funds or reduce the release amount.
    ReleaseAmountExceedsBalance = 21,

    // -----------------------------------------------------------------------
    // 30 – 39  Metadata errors
    // -----------------------------------------------------------------------

    /// The supplied IPFS CID byte string was either empty or exceeded the
    /// maximum allowed length (`MAX_CID_LEN` = 64 bytes).
    ///
    /// A valid CIDv1 base32 string (e.g. `bafybeig…`) is typically 59 chars
    /// for sha2-256 digests; anything outside the range `[1, 64]` is rejected.
    MetadataCidInvalid = 30,

    // -----------------------------------------------------------------------
    // 40 – 49  Access control errors
    // -----------------------------------------------------------------------

    /// The caller is not permitted to perform this operation.
    ///
    /// Context-specific meanings:
    /// - `update_metadata` — caller is not the project creator.
    /// - `set_fee_bps` / `set_treasury` — caller is not the protocol admin.
    CallerNotAuthorized = 40,

    /// The proposed fee in basis points exceeds the hard cap of 10 000
    /// (= 100 %).
    ///
    /// Pass a value in the range `[0, 10_000]`.
    FeeBpsExceedsMaximum = 41,
}
