// contracts/pifp_protocol/src/types.rs
//
// Multi-asset update:
//   - `Project.token: Address`   removed  (single-token field)
//   - `Project.balance: i128`    removed  (single-balance field)
//   - `Project.accepted_tokens`  added    (Vec<Address> whitelist, 1–10 tokens)
//   - `Project.total_raised_xlm` added    (informational aggregate, updated on deposit)
//
// Per-token balances are stored separately under DataKey::TokenBalance(project_id, token)
// so the Project struct itself stays a fixed size and avoids a Vec<(Address, i128)>
// that would grow with every donation.

use soroban_sdk::{contracttype, Address, Env, Vec};

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
}

/// A funding project stored on-chain.
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
    /// Ledger timestamp by which the project must be completed.
    pub deadline: u64,
    /// Current lifecycle state.
    pub status: ProjectStatus,
    /// Count of unique (token, donator) pairs that have donated.
    /// Informational; incremented on each new deposit.
    pub donation_count: u32,
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
    pub token:   Address,
    pub balance: i128,
}

/// Full balance view returned by `get_project_balances`.
#[contracttype]
#[derive(Clone, Debug)]
pub struct ProjectBalances {
    pub project_id: u64,
    pub balances:   Vec<TokenBalance>,
}