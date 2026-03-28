#![no_std]

// ---------------------------------------------------------------------------
// Modules
// ---------------------------------------------------------------------------
pub mod errors;

use errors::ContractError;
use soroban_sdk::{
    contract, contractimpl, contracttype, panic_with_error, symbol_short, Address, Bytes, BytesN,
    Env, Symbol,
};

// ---------------------------------------------------------------------------
// Storage keys
// ---------------------------------------------------------------------------
const CONFIG_KEY: Symbol = symbol_short!("CONFIG");

/// Maximum fee: 10 000 bps == 100 %.  Default: 100 bps == 1 %.
const MAX_FEE_BPS: u32 = 10_000;
const DEFAULT_FEE_BPS: u32 = 100; // 1 %

/// Maximum byte length for an IPFS CIDv1 base32 string.
const MAX_CID_LEN: u32 = 64;

// ---------------------------------------------------------------------------
// Data types
// ---------------------------------------------------------------------------

/// Global protocol configuration stored in instance storage.
///
/// Set once by `initialize()`; individual fields updated via admin methods.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtocolConfig {
    /// DAO / multisig admin that can adjust fee parameters.
    pub admin: Address,
    /// Treasury address that receives the protocol fee on every `release()`.
    pub treasury: Address,
    /// Fee in basis points (1 bps = 0.01 %).  100 bps = 1 %.
    pub fee_bps: u32,
}

/// On-chain project record.
///
/// Heavy metadata (name, description, images) lives off-chain at `metadata_cid`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Project {
    /// Address that created and administers the project.
    pub creator: Address,
    /// Funding target in the smallest token unit.
    pub goal: i128,
    /// Current balance deposited into this project.
    pub balance: i128,
    /// 32-byte content hash (e.g. Merkle root / sha256 of proof bundle) used
    /// to gate fund release.
    pub proof_hash: BytesN<32>,
    /// IPFS CIDv1 string (UTF-8 bytes, ≤64 bytes) pointing to the full project
    /// metadata JSON.
    pub metadata_cid: Bytes,
    /// Monotonically increasing counter bumped on every successful
    /// `update_metadata()` call.  Lets indexers detect stale caches.
    pub metadata_version: u32,
}

/// Lightweight read-only view of a [`Project`] returned by `get_project()`.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectInfo {
    pub creator: Address,
    pub goal: i128,
    pub balance: i128,
    pub proof_hash: BytesN<32>,
    pub metadata_cid: Bytes,
    pub metadata_version: u32,
}

/// Breakdown returned by `release()` so callers can verify the fee split
/// on-chain without re-computing it.
#[contracttype]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReleaseResult {
    /// Net amount routed to the project creator / beneficiary.
    pub recipient_amount: i128,
    /// Protocol fee routed to the DAO treasury.
    pub fee_amount: i128,
    /// Treasury address that received the fee (snapshot taken at call time).
    pub treasury: Address,
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn load_config(env: &Env) -> ProtocolConfig {
    env.storage()
        .instance()
        .get::<Symbol, ProtocolConfig>(&CONFIG_KEY)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::ProtocolNotInitialized))
}

fn save_config(env: &Env, config: &ProtocolConfig) {
    env.storage()
        .instance()
        .set::<Symbol, ProtocolConfig>(&CONFIG_KEY, config);
}

fn load_project(env: &Env, project_id: &BytesN<32>) -> Project {
    env.storage()
        .instance()
        .get::<BytesN<32>, Project>(project_id)
        .unwrap_or_else(|| panic_with_error!(env, ContractError::ProjectNotFound))
}

fn save_project(env: &Env, project_id: &BytesN<32>, project: &Project) {
    env.storage()
        .instance()
        .set::<BytesN<32>, Project>(project_id, project);
}

/// Reject CIDs that are empty or exceed the maximum allowed length.
fn validate_cid(env: &Env, cid: &Bytes) {
    if cid.len() == 0 || cid.len() > MAX_CID_LEN {
        panic_with_error!(env, ContractError::MetadataCidInvalid);
    }
}

/// Compute fee split.  Returns `(recipient_amount, fee_amount)`.
///
/// `fee_amount      = floor(amount × fee_bps / 10_000)`
/// `recipient_amount = amount − fee_amount`
///
/// Both values are guaranteed to be non-negative when `amount > 0` and
/// `fee_bps ≤ MAX_FEE_BPS`.
pub fn compute_fee_split(amount: i128, fee_bps: u32) -> (i128, i128) {
    let fee = (amount * fee_bps as i128) / MAX_FEE_BPS as i128;
    (amount - fee, fee)
}

// ---------------------------------------------------------------------------
// Contract
// ---------------------------------------------------------------------------

#[contract]
pub struct PifpProtocol;

#[contractimpl]
impl PifpProtocol {
    // -----------------------------------------------------------------------
    // Initialisation  (called once by deployer)
    // -----------------------------------------------------------------------

    /// Initialise the protocol with an admin, treasury, and optional fee.
    ///
    /// **Must be called exactly once** immediately after deployment.
    /// Subsequent calls will panic with
    /// [`ContractError::ProtocolAlreadyInitialized`].
    ///
    /// # Parameters
    /// - `admin`   — DAO multisig address that controls fee parameters.
    /// - `treasury`— Address that receives the protocol fee on every `release`.
    /// - `fee_bps` — Initial fee in basis points; pass `None` to use the 1 %
    ///               default (100 bps).
    pub fn initialize(env: Env, admin: Address, treasury: Address, fee_bps: Option<u32>) {
        admin.require_auth();

        if env.storage().instance().has::<Symbol>(&CONFIG_KEY) {
            panic_with_error!(&env, ContractError::ProtocolAlreadyInitialized);
        }

        let bps = fee_bps.unwrap_or(DEFAULT_FEE_BPS);
        if bps > MAX_FEE_BPS {
            panic_with_error!(&env, ContractError::FeeBpsExceedsMaximum);
        }

        save_config(&env, &ProtocolConfig { admin, treasury, fee_bps: bps });
    }

    /// Return the current [`ProtocolConfig`].
    pub fn get_config(env: Env) -> ProtocolConfig {
        load_config(&env)
    }

    // -----------------------------------------------------------------------
    // Admin: fee / treasury management
    // -----------------------------------------------------------------------

    /// Update the protocol fee (basis points).
    ///
    /// Panics with [`ContractError::CallerNotAuthorized`] if `caller` is not
    /// the registered admin.  Panics with [`ContractError::FeeBpsExceedsMaximum`]
    /// if `new_fee_bps > 10_000`.
    ///
    /// Emits a `FEE_UPD` event carrying `(old_bps, new_bps)`.
    pub fn set_fee_bps(env: Env, caller: Address, new_fee_bps: u32) {
        caller.require_auth();

        let mut config = load_config(&env);
        if config.admin != caller {
            panic_with_error!(&env, ContractError::CallerNotAuthorized);
        }
        if new_fee_bps > MAX_FEE_BPS {
            panic_with_error!(&env, ContractError::FeeBpsExceedsMaximum);
        }

        let old_bps = config.fee_bps;
        config.fee_bps = new_fee_bps;
        save_config(&env, &config);

        env.events().publish((symbol_short!("FEE_UPD"),), (old_bps, new_fee_bps));
    }

    /// Rotate the treasury address.
    ///
    /// Panics with [`ContractError::CallerNotAuthorized`] if `caller` is not
    /// the registered admin.
    ///
    /// Emits a `TRES_UPD` event carrying `(old_treasury, new_treasury)`.
    pub fn set_treasury(env: Env, caller: Address, new_treasury: Address) {
        caller.require_auth();

        let mut config = load_config(&env);
        if config.admin != caller {
            panic_with_error!(&env, ContractError::CallerNotAuthorized);
        }

        let old_treasury = config.treasury.clone();
        config.treasury = new_treasury.clone();
        save_config(&env, &config);

        env.events().publish((symbol_short!("TRES_UPD"),), (old_treasury, new_treasury));
    }

    // -----------------------------------------------------------------------
    // Project lifecycle
    // -----------------------------------------------------------------------

    /// Register a new project on-chain.
    ///
    /// Requires the protocol to be initialised.  Panics with:
    /// - [`ContractError::ProjectGoalNotPositive`] — if `goal ≤ 0`.
    /// - [`ContractError::MetadataCidInvalid`]     — if `metadata_cid` is
    ///   empty or exceeds 64 bytes.
    pub fn create_project(
        env: Env,
        project_id: BytesN<32>,
        creator: Address,
        goal: i128,
        proof_hash: BytesN<32>,
        metadata_cid: Bytes,
    ) -> ProjectInfo {
        creator.require_auth();
        // Ensure the protocol has been initialised before any project is created.
        load_config(&env);

        if goal <= 0 {
            panic_with_error!(&env, ContractError::ProjectGoalNotPositive);
        }
        validate_cid(&env, &metadata_cid);

        let project = Project {
            creator: creator.clone(),
            goal,
            balance: 0,
            proof_hash: proof_hash.clone(),
            metadata_cid: metadata_cid.clone(),
            metadata_version: 0,
        };
        save_project(&env, &project_id, &project);

        ProjectInfo { creator, goal, balance: 0, proof_hash, metadata_cid, metadata_version: 0 }
    }

    /// Retrieve a project's current state.
    ///
    /// Panics with [`ContractError::ProjectNotFound`] if `project_id` is unknown.
    pub fn get_project(env: Env, project_id: BytesN<32>) -> ProjectInfo {
        let p = load_project(&env, &project_id);
        ProjectInfo {
            creator: p.creator,
            goal: p.goal,
            balance: p.balance,
            proof_hash: p.proof_hash,
            metadata_cid: p.metadata_cid,
            metadata_version: p.metadata_version,
        }
    }

    // -----------------------------------------------------------------------
    // Metadata management
    // -----------------------------------------------------------------------

    /// Update the IPFS CID pointing to the project's extended metadata JSON.
    ///
    /// Only the project creator may call this.  Panics with:
    /// - [`ContractError::MetadataCidInvalid`]   — invalid CID.
    /// - [`ContractError::CallerNotAuthorized`]  — caller ≠ project creator.
    ///
    /// Emits `META_UPD` event carrying `(new_cid, new_version)`.
    /// Returns the new `metadata_version`.
    pub fn update_metadata(
        env: Env,
        project_id: BytesN<32>,
        caller: Address,
        new_metadata_cid: Bytes,
    ) -> u32 {
        caller.require_auth();
        validate_cid(&env, &new_metadata_cid);

        let mut project = load_project(&env, &project_id);
        if project.creator != caller {
            panic_with_error!(&env, ContractError::CallerNotAuthorized);
        }

        project.metadata_cid = new_metadata_cid.clone();
        project.metadata_version += 1;
        save_project(&env, &project_id, &project);

        env.events().publish(
            (symbol_short!("META_UPD"), project_id),
            (new_metadata_cid, project.metadata_version),
        );

        project.metadata_version
    }

    // -----------------------------------------------------------------------
    // Funding operations
    // -----------------------------------------------------------------------

    /// Deposit funds into a project.
    ///
    /// Updates the project's on-chain balance.  Real token transfer (via SAC)
    /// will be wired here in a future milestone.
    ///
    /// Panics with [`ContractError::ProjectNotFound`] if the project does not
    /// exist.
    pub fn deposit(env: Env, _donor: Address, project_id: BytesN<32>, amount: i128) {
        let mut project = load_project(&env, &project_id);
        project.balance += amount;
        save_project(&env, &project_id, &project);
    }

    /// Verify submitted proof against the stored `proof_hash` (stub).
    ///
    /// A future implementation will:
    /// - Hash-compare `submitted_proof_hash` against `project.proof_hash`.
    /// - Enforce oracle attestations / milestone gates.
    pub fn verify(
        _env: Env,
        _project_id: BytesN<32>,
        _submitted_proof_hash: BytesN<32>,
    ) {
        // TODO: hash comparison + oracle attestation logic.
    }

    /// Withdraw released funds from a project.
    ///
    /// Panics with [`ContractError::ProjectNotFound`] if the project does not
    /// exist.  Real token transfer (via SAC) and verification gate will be
    /// added in a future milestone.
    pub fn withdraw(env: Env, _recipient: Address, project_id: BytesN<32>, amount: i128) {
        let mut project = load_project(&env, &project_id);
        project.balance -= amount;
        save_project(&env, &project_id, &project);
    }

    /// Verify proof of impact **and** release funds with protocol fee split.
    ///
    /// # Fee mechanics
    /// ```text
    /// fee_amount       = floor(amount × fee_bps / 10_000)
    /// recipient_amount = amount − fee_amount
    /// ```
    ///
    /// # Panics
    /// - [`ContractError::ReleaseAmountIsZero`]  — `amount ≤ 0`.
    /// - [`ContractError::ProtocolNotInitialized`] — protocol not set up.
    /// - [`ContractError::ProjectNotFound`]       — unknown project ID.
    ///
    /// # Events
    /// Emits `RELEASE` with topic `(project_id,)` and data
    /// `(recipient, recipient_amount, treasury, fee_amount, fee_bps)`.
    ///
    /// # Returns
    /// [`ReleaseResult`] containing the split amounts and treasury snapshot.
    pub fn release(
        env: Env,
        project_id: BytesN<32>,
        submitted_proof_hash: BytesN<32>,
        recipient: Address,
        amount: i128,
    ) -> ReleaseResult {
        if amount <= 0 {
            panic_with_error!(&env, ContractError::ReleaseAmountIsZero);
        }

        let config = load_config(&env);
        let mut project = load_project(&env, &project_id);

        // Proof verification stub — real logic comes with oracle integration.
        let _ = submitted_proof_hash;

        let (recipient_amount, fee_amount) = compute_fee_split(amount, config.fee_bps);

        project.balance -= amount;
        save_project(&env, &project_id, &project);

        env.events().publish(
            (symbol_short!("RELEASE"), project_id),
            (
                recipient.clone(),
                recipient_amount,
                config.treasury.clone(),
                fee_amount,
                config.fee_bps,
            ),
        );

        ReleaseResult { recipient_amount, fee_amount, treasury: config.treasury }
    }

    /// Legacy stub kept for backwards compatibility.
    ///
    /// **Deprecated** — use `release()` which applies the treasury fee split.
    pub fn verify_and_release(
        _env: Env,
        _project_id: BytesN<32>,
        _submitted_proof_hash: BytesN<32>,
    ) {
        // Deprecated: use `release` which correctly deducts the protocol fee.
    }
}

// ---------------------------------------------------------------------------
// Fee math unit tests (no contract / host needed)
// ---------------------------------------------------------------------------
#[cfg(test)]
mod fee_math_tests {
    use super::compute_fee_split;

    #[test]
    fn test_1_percent_fee() {
        let (recipient, fee) = compute_fee_split(10_000, 100);
        assert_eq!(fee, 100);
        assert_eq!(recipient, 9_900);
        assert_eq!(recipient + fee, 10_000);
    }

    #[test]
    fn test_zero_fee() {
        let (recipient, fee) = compute_fee_split(5_000, 0);
        assert_eq!(fee, 0);
        assert_eq!(recipient, 5_000);
    }

    #[test]
    fn test_50_percent_fee() {
        let (recipient, fee) = compute_fee_split(1_000, 5_000);
        assert_eq!(fee, 500);
        assert_eq!(recipient, 500);
    }

    #[test]
    fn test_100_percent_fee() {
        let (recipient, fee) = compute_fee_split(1_000, 10_000);
        assert_eq!(fee, 1_000);
        assert_eq!(recipient, 0);
    }

    #[test]
    fn test_fee_floor_rounding() {
        // 1 unit at 1 % → fee = floor(1 * 100 / 10_000) = 0
        let (recipient, fee) = compute_fee_split(1, 100);
        assert_eq!(fee, 0);
        assert_eq!(recipient, 1);
    }
}

// ---------------------------------------------------------------------------
// Integration tests
// ---------------------------------------------------------------------------
#[cfg(test)]
mod bench_test;

#[cfg(test)]
mod tests {
    use super::*;
    use soroban_sdk::{testutils::Address as _, Address, Bytes, BytesN, Env};

    fn make_cid(env: &Env) -> Bytes {
        Bytes::from_slice(env, b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi")
    }

    fn make_project_id(env: &Env, seed: u8) -> BytesN<32> {
        BytesN::from_array(env, &[seed; 32])
    }

    fn setup(env: &Env) -> (PifpProtocolClient, Address, Address) {
        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(env, &contract_id);
        let admin = Address::generate(env);
        let treasury = Address::generate(env);
        client.initialize(&admin, &treasury, &None);
        (client, admin, treasury)
    }

    // -----------------------------------------------------------------------
    // Error code identity checks
    // -----------------------------------------------------------------------

    /// Asserts that every variant maps to the documented numeric code so that
    /// changes to the numbering are caught immediately.
    #[test]
    fn test_error_codes_are_stable() {
        assert_eq!(ContractError::ProtocolAlreadyInitialized as u32, 1);
        assert_eq!(ContractError::ProtocolNotInitialized     as u32, 2);
        assert_eq!(ContractError::ProjectNotFound            as u32, 10);
        assert_eq!(ContractError::ProjectGoalNotPositive     as u32, 11);
        assert_eq!(ContractError::ReleaseAmountIsZero        as u32, 20);
        assert_eq!(ContractError::ReleaseAmountExceedsBalance as u32, 21);
        assert_eq!(ContractError::MetadataCidInvalid         as u32, 30);
        assert_eq!(ContractError::CallerNotAuthorized        as u32, 40);
        assert_eq!(ContractError::FeeBpsExceedsMaximum       as u32, 41);
    }

    // -----------------------------------------------------------------------
    // Initialisation tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_initialize_stores_config() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, treasury) = setup(&env);

        let cfg = client.get_config();
        assert_eq!(cfg.admin, admin);
        assert_eq!(cfg.treasury, treasury);
        assert_eq!(cfg.fee_bps, 100);
    }

    #[test]
    #[should_panic]
    fn test_double_initialize_panics() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, treasury) = setup(&env);
        client.initialize(&admin, &treasury, &None); // ProtocolAlreadyInitialized
    }

    // -----------------------------------------------------------------------
    // Fee / treasury admin tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_set_fee_bps_updates_config() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup(&env);

        client.set_fee_bps(&admin, &250);
        assert_eq!(client.get_config().fee_bps, 250);
    }

    #[test]
    #[should_panic]
    fn test_set_fee_bps_too_high_panics() {
        // FeeBpsExceedsMaximum
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup(&env);
        client.set_fee_bps(&admin, &10_001);
    }

    #[test]
    #[should_panic]
    fn test_set_fee_bps_non_admin_panics() {
        // CallerNotAuthorized
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);
        let impostor = Address::generate(&env);
        client.set_fee_bps(&impostor, &200);
    }

    #[test]
    fn test_set_treasury_updates_address() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, admin, _) = setup(&env);

        let new_treasury = Address::generate(&env);
        client.set_treasury(&admin, &new_treasury);
        assert_eq!(client.get_config().treasury, new_treasury);
    }

    #[test]
    #[should_panic]
    fn test_set_treasury_non_admin_panics() {
        // CallerNotAuthorized
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);
        let impostor = Address::generate(&env);
        client.set_treasury(&impostor, &Address::generate(&env));
    }

    // -----------------------------------------------------------------------
    // Release / fee-split tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_release_splits_fee_correctly() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, treasury) = setup(&env); // 1 % fee

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 20);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(&pid, &creator, &10_000i128, &proof_hash, &make_cid(&env));
        client.deposit(&creator, &pid, &10_000i128);

        let result = client.release(&pid, &proof_hash, &creator, &10_000i128);
        assert_eq!(result.fee_amount, 100);
        assert_eq!(result.recipient_amount, 9_900);
        assert_eq!(result.treasury, treasury);

        let info = client.get_project(&pid);
        assert_eq!(info.balance, 0);
    }

    #[test]
    fn test_release_with_zero_fee() {
        let env = Env::default();
        env.mock_all_auths();
        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(&env, &contract_id);
        let admin = Address::generate(&env);
        let treasury = Address::generate(&env);
        client.initialize(&admin, &treasury, &Some(0u32));

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 21);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(&pid, &creator, &5_000i128, &proof_hash, &make_cid(&env));
        client.deposit(&creator, &pid, &5_000i128);

        let result = client.release(&pid, &proof_hash, &creator, &5_000i128);
        assert_eq!(result.fee_amount, 0);
        assert_eq!(result.recipient_amount, 5_000);
    }

    #[test]
    #[should_panic]
    fn test_release_zero_amount_panics() {
        // ReleaseAmountIsZero
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 22);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(&pid, &creator, &5_000i128, &proof_hash, &make_cid(&env));
        client.release(&pid, &proof_hash, &creator, &0i128);
    }

    // -----------------------------------------------------------------------
    // Metadata tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_create_project_stores_metadata_cid() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 1);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        let cid = make_cid(&env);

        let info = client.create_project(&pid, &creator, &10_000i128, &proof_hash, &cid);
        assert_eq!(info.creator, creator);
        assert_eq!(info.goal, 10_000);
        assert_eq!(info.balance, 0);
        assert_eq!(info.metadata_cid, cid);
        assert_eq!(info.metadata_version, 0);
    }

    #[test]
    fn test_update_metadata_bumps_version_and_cid() {
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 2);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(&pid, &creator, &5_000i128, &proof_hash, &make_cid(&env));

        let new_cid =
            Bytes::from_slice(&env, b"bafkreigh2akiscaildcqabab4eupks44qq6y2plwqpk3mvkvbgm7qjlxp4");
        let version = client.update_metadata(&pid, &creator, &new_cid);
        assert_eq!(version, 1);

        let info = client.get_project(&pid);
        assert_eq!(info.metadata_cid, new_cid);
        assert_eq!(info.metadata_version, 1);
    }

    #[test]
    #[should_panic]
    fn test_update_metadata_rejects_non_creator() {
        // CallerNotAuthorized
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);

        let creator = Address::generate(&env);
        let attacker = Address::generate(&env);
        let pid = make_project_id(&env, 3);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(&pid, &creator, &1_000i128, &proof_hash, &make_cid(&env));
        client.update_metadata(
            &pid,
            &attacker,
            &Bytes::from_slice(
                &env,
                b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
            ),
        );
    }

    #[test]
    #[should_panic]
    fn test_create_project_rejects_empty_cid() {
        // MetadataCidInvalid
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 4);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(
            &pid,
            &creator,
            &1_000i128,
            &proof_hash,
            &Bytes::from_slice(&env, b""),
        );
    }

    #[test]
    #[should_panic]
    fn test_create_project_rejects_negative_goal() {
        // ProjectGoalNotPositive
        let env = Env::default();
        env.mock_all_auths();
        let (client, _, _) = setup(&env);

        let creator = Address::generate(&env);
        let pid = make_project_id(&env, 5);
        let proof_hash = BytesN::from_array(&env, &[0u8; 32]);
        client.create_project(&pid, &creator, &-100i128, &proof_hash, &make_cid(&env));
    }
}
