#![no_std]

use soroban_sdk::{contract, contractimpl, Address, BytesN, Env};

mod storage;
mod types;

#[cfg(test)]
mod test;

use storage::{get_and_increment_project_id, load_project, save_project};
pub use types::{Project, ProjectStatus};
#[cfg(test)]
mod test;

use soroban_sdk::{
    contract, contracterror, contractimpl, contracttype, panic_with_error, token, Address, BytesN,
    Env, Symbol, Vec,
};

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MilestoneStatus {
    Pending = 0,
    Released = 1,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Milestone {
    pub id: u32,
    pub amount: i128,
    pub status: MilestoneStatus,
}

#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    pub id: BytesN<32>,
    pub creator: Address,
    pub oracle: Address,
    pub token: Address,
    pub goal: i128,
    pub milestones: Vec<Milestone>,
    pub balance: i128,
}

#[contracttype]
pub enum DataKey {
    Project(BytesN<32>),
}

#[contracterror]
#[derive(Copy, Clone, Debug, Eq, PartialEq, PartialOrd, Ord)]
#[repr(u32)]
pub enum Error {
    ProjectNotFound = 1,
    MilestoneNotFound = 2,
    MilestoneAlreadyReleased = 3,
    InsufficientBalance = 4,
    InvalidMilestones = 5,
    NotAuthorized = 6,
    GoalMismatch = 7,
}

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {
    /// Register a new funding project.
    ///
    /// - `creator` must authorize the call.
    /// - `goal` is the target funding amount (must be > 0).
    /// - `proof_hash` is a content hash (e.g. IPFS CID digest) representing proof artifacts.
    /// - `deadline` is a ledger timestamp by which the project must be completed (must be in the future).
    ///
    /// Returns the persisted `Project` with a unique auto-incremented `id`.
    pub fn register_project(
        env: Env,
        creator: Address,
        goal: i128,
        proof_hash: BytesN<32>,
        deadline: u64,
    ) -> Project {
        creator.require_auth();

        if goal <= 0 {
            panic_with_error!(&env, Error::InvalidMilestones);
        }

        if deadline <= env.ledger().timestamp() {
            panic!("deadline must be in the future");
        }

        let id = get_and_increment_project_id(&env);

        let project = Project {
            id,
            creator,
            goal,
            balance: 0,
            proof_hash,
            deadline,
            status: ProjectStatus::Funding,
        };

        save_project(&env, &project);

        project
    }

    /// Retrieve a project by its ID.
    ///
    /// Panics if the project does not exist.
    pub fn get_project(env: Env, id: u64) -> Project {
        load_project(&env, id)
    }

    /// Deposit funds into a project.
    pub fn deposit(env: Env, project_id: BytesN<32>, donator: Address, amount: i128) {
        donator.require_auth();

        let mut project = Self::get_project(env.clone(), project_id.clone())
            .unwrap_or_else(|| panic_with_error!(&env, Error::ProjectNotFound));

        // Transfer tokens from donator to contract
        let token_client = token::Client::new(&env, &project.token);
        token_client.transfer(&donator, &env.current_contract_address(), &amount);

        project.balance += amount;
        env.storage()
            .persistent()
            .set(&DataKey::Project(project_id.clone()), &project);

        // Emit donation event
        env.events().publish(
            (Symbol::new(&env, "donation_received"), project_id),
            (donator, amount),
        );
    }

    /// Release funds for a specific milestone.
    ///
    /// NOTE: This is a skeleton. A real implementation would:
    /// - Load the project from the registry via `get_project`
    /// - Verify the submitted proof against `proof_hash`
    /// - Enforce milestones / attestations / oracle signatures
    /// - Transfer/release funds and update balances / status
    pub fn verify_and_release(_env: Env, _project_id: u64, _submitted_proof_hash: BytesN<32>) {
        // TODO: implement verification logic and release mechanism.
    }
}
