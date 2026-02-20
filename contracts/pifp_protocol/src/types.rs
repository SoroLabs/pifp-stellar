use soroban_sdk::{contracttype, Address, BytesN};

/// Lifecycle status of a project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProjectStatus {
    /// Accepting donations.
    Funding,
    /// Fully funded; work in progress.
    Active,
    /// Proof verified; funds released.
    Completed,
    /// Deadline passed without completion.
    Expired,
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
    pub token: Address,
    pub goal: i128,
    pub proof_hash: BytesN<32>,
    pub deadline: u64,
}

/// Mutable project state, updated on deposits and verification.
///
/// Kept small (~20 bytes) so that frequent writes (deposits) are cheap.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectState {
    pub balance: i128,
    pub status: ProjectStatus,
}

/// Full on-chain representation of a funding project.
///
/// Used as the public API return type; reconstructed internally from
/// the split `ProjectConfig` + `ProjectState` storage entries.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Unique identifier (auto-incremented).
    pub id: u64,
    /// Address that created the project and receives funds.
    pub creator: Address,
    /// Address of the token representing the funding asset.
    pub token: Address,
    /// Target funding amount.
    pub goal: i128,
    /// Current funded amount.
    pub balance: i128,
    /// Content-hash representing proof artifacts (e.g. IPFS CID digest).
    pub proof_hash: BytesN<32>,
    /// Ledger timestamp by which the project must be completed.
    pub deadline: u64,
    /// Current lifecycle status.
    pub status: ProjectStatus,
}
