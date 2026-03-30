//! # Error Catalogue
//!
//! Every error the PIFP protocol contract can return is defined here as a
//! [`contracterror`] enum. Soroban surfaces these to callers as
//! `Error(Contract, #N)` where `N` is the discriminant value.
//!
//! ## Error codes at a glance
//!
//! | Code | Variant                  | Typical trigger                                             |
//! |------|--------------------------|-------------------------------------------------------------|
//! |  1   | `ProjectNotFound`        | Querying or operating on a project ID that does not exist   |
//! |  2   | `MilestoneNotFound`      | Reserved for future milestone-level operations              |
//! |  3   | `MilestoneAlreadyReleased` | Calling `verify_proof` on an already-verified/completed project |
//! |  4   | `InsufficientBalance`    | Refund requested but donator has zero balance for that token |
//! |  5   | `InvalidMilestones`      | Reserved for future milestone validation                    |
//! |  6   | `NotAuthorized`          | Caller lacks the RBAC role required for the operation       |
//! |  7   | `InvalidGoal`            | Goal is ≤ 0 or exceeds the 10^30 upper bound               |
//! |  8   | `AlreadyInitialized`     | `init` called more than once                                |
//! |  9   | `RoleNotFound`           | Reserved for role-query edge cases                          |
//! | 10   | `TooManyTokens`          | `accepted_tokens` list exceeds the 10-token cap             |
//! | 11   | `InvalidAmount`          | Deposit or transfer amount is ≤ 0                           |
//! | 12   | `DuplicateToken`         | `accepted_tokens` contains the same address twice           |
//! | 13   | `InvalidDeadline`        | Deadline is in the past or more than 5 years in the future  |
//! | 14   | `ProjectExpired`         | Operation attempted on a project whose deadline has passed  |
//! | 15   | `ProjectNotActive`       | Deposit/verify attempted on a Completed or invalid-status project |
//! | 16   | `VerificationFailed`     | Submitted proof hash does not match the stored proof hash   |
//! | 17   | `EmptyAcceptedTokens`    | `accepted_tokens` list is empty at registration             |
//! | 18   | `Overflow`               | Arithmetic overflow on balance addition                     |
//! | 19   | `ProtocolPaused`         | Mutating operation attempted while the protocol is paused   |
//! | 20   | `GoalMismatch`           | Reserved for cross-token goal validation                    |
//! | 21   | `ProjectNotExpired`      | Refund or expire attempted before the deadline has passed   |
//! | 22   | `InvalidTransition`      | State-machine transition not allowed (e.g. expiring a Completed project) |
//! | 23   | `TokenNotAccepted`       | Deposit attempted with a token not in the project's accepted list |
//! | 24   | `DeadlineTooLong`        | The new deadline exceeds the 1-year extension limit |
//! | 25   | `InvalidFeeBasisPoints`  | Fee basis points exceed the maximum allowed (10%). |
//! | 26   | `NotWhitelisted`         | Address is not on the project's whitelist. |
//! | 27   | `RefundWindowActive`     | Creator tried to reclaim funds before the 6-month refund window expired |
//! | 28   | `RefundWindowExpired`    | Donor tried to refund after the 6-month refund window expired |
//! | 29   | `ProtocolNotInitialized` | Contract state has not been initialized |
//! | 30   | `ReleaseAmountExceedsBalance` | The requested release amount exceeds the project's current on-chain balance |
//! | 31   | `MetadataCidInvalid`     | IPFS CID byte string was empty or exceeded max length |
//! | 32   | `FeeBpsExceedsMaximum`   | Configured fee in basis points exceeds the 10_000 hard cap |
//! | 33   | `ProjectPaused`          | Mutating project action attempted while the project is paused |
//! | 34   | `GracePeriodActive`      | `claim_funds` called before the 24-hour grace period has elapsed |
//! | 35   | `ReentrancyDetected`     | A re-entrant call was detected; the contract is already executing |
//! | 36   | `InvalidOracleConfig`    | Oracle threshold or count is invalid. |

use soroban_sdk::contracterror;

/// All contract-level errors returned by the PIFP protocol.
#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    /// The requested project ID does not exist in storage.
    ProjectNotFound = 1,

    /// The requested milestone index is out of bounds.
    MilestoneNotFound = 2,

    /// Verification called on a project that is already verified or completed.
    MilestoneAlreadyReleased = 3,

    /// The donator has no refundable balance for the requested token.
    InsufficientBalance = 4,

    /// Milestone validation failed (e.g., total BPS != 10,000).
    InvalidMilestones = 5,

    /// The caller does not hold the RBAC role required for this operation.
    NotAuthorized = 6,

    /// The funding goal is ≤ 0 or exceeds the protocol's upper bound (10^30).
    InvalidGoal = 7,

    /// `init` has already been called; the SuperAdmin is already set.
    AlreadyInitialized = 8,

    /// Reserved — the queried address holds no RBAC role.
    RoleNotFound = 9,

    /// The `accepted_tokens` list exceeds the maximum of 10 tokens.
    TooManyTokens = 10,

    /// A deposit or transfer amount is ≤ 0.
    InvalidAmount = 11,

    /// The `accepted_tokens` list contains duplicate token addresses.
    DuplicateToken = 12,

    /// The deadline is in the past or more than 5 years in the future.
    InvalidDeadline = 13,

    /// The project's deadline has passed.
    ProjectExpired = 14,

    /// The project is not in `Funding` or `Active` status.
    ProjectNotActive = 15,

    /// The submitted proof hash does not match the project's stored `proof_hash`.
    VerificationFailed = 16,

    /// Registration attempted with an empty `accepted_tokens` list.
    EmptyAcceptedTokens = 17,

    /// Arithmetic overflow.
    Overflow = 18,

    /// The protocol is currently paused.
    ProtocolPaused = 19,

    /// Reserved — cross-token goal validation mismatch.
    GoalMismatch = 20,

    /// Refund or explicit expiration attempted before the project deadline.
    ProjectNotExpired = 21,

    /// The requested status transition is not allowed.
    InvalidTransition = 22,

    /// The deposit token is not in the project's `accepted_tokens` list.
    TokenNotAccepted = 23,

    /// The new deadline exceeds the extension limits.
    DeadlineTooLong = 24,

    /// Fee basis points exceed the maximum allowed.
    InvalidFeeBasisPoints = 25,

    /// Address is not on the project's whitelist.
    NotWhitelisted = 26,

    /// The donor refund window is still active.
    RefundWindowActive = 27,

    /// The donor refund window has expired.
    RefundWindowExpired = 28,

    /// Contract state has not been initialized.
    ProtocolNotInitialized = 29,

    /// The requested release amount exceeds the project's current balance.
    ReleaseAmountExceedsBalance = 30,

    /// The supplied IPFS CID byte string was invalid.
    MetadataCidInvalid = 31,

    /// The proposed fee in basis points exceeds the hard cap.
    FeeBpsExceedsMaximum = 32,

    /// The target project is paused.
    ProjectPaused = 33,

    /// The 24-hour grace period after proof verification has not yet elapsed.
    GracePeriodActive = 34,

    /// A re-entrant call was detected.
    ReentrancyDetected = 35,

    /// Oracle threshold or count is invalid.
    InvalidOracleConfig = 36,
}
