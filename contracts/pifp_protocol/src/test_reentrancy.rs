//! # Re-entrancy Guard Tests (INV-11)
//!
//! Verifies that `check_no_recursive_state` blocks any call that arrives while
//! the re-entrancy lock is held.  Because Soroban's host does not support true
//! cross-contract callbacks within a single test invocation, we simulate the
//! attack by directly setting the `IsLocked` flag in storage before calling a
//! sensitive entry point — exactly the state a malicious token contract would
//! produce if it called back into PIFP during a `transfer`.

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, Bytes, BytesN, Env,
};

use crate::{PifpProtocol, PifpProtocolClient, Role};

// ── Shared setup ─────────────────────────────────────────────────────

struct Ctx {
    env: Env,
    client: PifpProtocolClient<'static>,
    admin: Address,
    oracle: Address,
    manager: Address,
}

impl Ctx {
    fn new() -> Self {
        let env = Env::default();
        env.mock_all_auths();
        let mut ledger = env.ledger().get();
        ledger.timestamp = 100_000;
        env.ledger().set(ledger);

        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let manager = Address::generate(&env);

        client.init(&admin);
        client.grant_role(&admin, &oracle, &Role::Oracle);
        client.grant_role(&admin, &manager, &Role::ProjectManager);

        Self { env, client, admin, oracle, manager }
    }

    fn create_token(&self) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
        let addr = self.env.register_stellar_asset_contract_v2(self.admin.clone());
        (
            token::Client::new(&self.env, &addr.address()),
            token::StellarAssetClient::new(&self.env, &addr.address()),
        )
    }

    fn dummy_proof(&self) -> BytesN<32> {
        BytesN::from_array(&self.env, &[0xabu8; 32])
    }

    fn dummy_uri(&self) -> Bytes {
        Bytes::from_slice(
            &self.env,
            b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi",
        )
    }

    fn register(&self, token_addr: &Address, goal: i128) -> u64 {
        let tokens = soroban_sdk::vec![&self.env, token_addr.clone()];
        let deadline = self.env.ledger().timestamp() + 86_400;
        let p = self.client.register_project(
            &self.manager,
            &tokens,
            &goal,
            &self.dummy_proof(),
            &self.dummy_uri(),
            &deadline,
            &false,
            &0u32,
        );
        p.id
    }

    /// Simulate a re-entrant state by setting the lock directly in storage.
    fn force_lock(&self) {
        let contract_id = self.client.address.clone();
        self.env.as_contract(&contract_id, || {
            crate::storage::set_locked(&self.env, true);
        });
    }

    fn jump_time(&self, secs: u64) {
        let mut ledger = self.env.ledger().get();
        ledger.timestamp += secs;
        self.env.ledger().set(ledger);
    }
}

// ── deposit blocked when locked ───────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_deposit_blocked_when_locked() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &500);

    // Simulate re-entrant state.
    ctx.force_lock();

    ctx.client.deposit(&project_id, &ctx.manager, &token.address, &500i128);
}

// ── verify_and_release blocked when locked ────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_verify_and_release_blocked_when_locked() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &1_000);
    ctx.client.deposit(&project_id, &ctx.manager, &token.address, &1_000i128);

    // Simulate re-entrant state.
    ctx.force_lock();

    ctx.client.verify_and_release(&ctx.oracle, &project_id, &ctx.dummy_proof());
}

// ── refund blocked when locked ────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_refund_blocked_when_locked() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &200);
    ctx.client.deposit(&project_id, &ctx.manager, &token.address, &200i128);

    // Expire the project so refund is valid.
    ctx.jump_time(86_401);
    ctx.client.expire_project(&project_id);

    // Simulate re-entrant state.
    ctx.force_lock();

    ctx.client.refund(&ctx.manager, &project_id, &token.address);
}

// ── lock is released after a normal deposit ───────────────────────────

#[test]
fn test_lock_released_after_successful_deposit() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &500);
    ctx.client.deposit(&project_id, &ctx.manager, &token.address, &500i128);

    // Lock must be cleared after the call completes.
    let contract_id = ctx.client.address.clone();
    ctx.env.as_contract(&contract_id, || {
        assert!(!crate::storage::is_locked(&ctx.env));
    });
}
