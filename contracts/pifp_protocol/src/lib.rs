#![no_std]

use soroban_sdk::{contract, contractimpl, symbol_short, Address, BytesN, Env};

mod storage;
mod types;

#[cfg(test)]
mod test;

use storage::{get_and_increment_project_id, get_oracle, load_project, save_project, set_oracle};
pub use types::{Project, ProjectStatus};

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

    /// Set the trusted oracle/verifier address.
    ///
    /// - `admin` must authorize the call (the caller setting the oracle).
    /// - `oracle` is the address that will be permitted to verify proofs.
    pub fn set_oracle(env: Env, admin: Address, oracle: Address) {
        admin.require_auth();
        set_oracle(&env, &oracle);
    }

    /// Verify proof of impact and update project status.
    ///
    /// The registered oracle submits a proof hash. If it matches the project's
    /// stored `proof_hash`, the project status transitions to `Completed`.
    ///
    /// NOTE: This is a mocked verification (hash equality).
    /// The structure is prepared for future ZK-STARK verification.
    ///
    /// - Only the registered oracle may call this.
    /// - The project must be in `Funding` or `Active` status.
    /// - `submitted_proof_hash` must match the project's `proof_hash`.
    pub fn verify_and_release(env: Env, project_id: u64, submitted_proof_hash: BytesN<32>) {
        // Ensure caller is the registered oracle.
        let oracle = get_oracle(&env);
        oracle.require_auth();

        // Load the project.
        let mut project = load_project(&env, project_id);

        // Ensure the project is in a verifiable state.
        match project.status {
            ProjectStatus::Funding | ProjectStatus::Active => {}
            ProjectStatus::Completed => panic!("project already completed"),
            ProjectStatus::Expired => panic!("project has expired"),
        }

        // Mocked ZK verification: compare submitted hash to stored hash.
        if submitted_proof_hash != project.proof_hash {
            panic!("proof verification failed: hash mismatch");
        }

        // Transition to Completed.
        project.status = ProjectStatus::Completed;
        save_project(&env, &project);

        // Emit verification event.
        env.events()
            .publish((symbol_short!("verified"),), project_id);
    }
}
