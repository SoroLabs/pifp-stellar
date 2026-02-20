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

/// On-chain representation of a funding project.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Unique identifier (auto-incremented).
    pub id: u64,
    /// Address that created the project and receives funds.
    pub creator: Address,
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
