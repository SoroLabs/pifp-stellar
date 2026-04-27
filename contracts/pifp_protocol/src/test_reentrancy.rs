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
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Bytes, BytesN, Env, IntoVal, Val, Vec,
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
        let mut ledger = env.ledger().get();
        ledger.timestamp = 100_000;
        env.ledger().set(ledger);

        let contract_id = env.register(PifpProtocol, ());
        let client = PifpProtocolClient::new(&env, &contract_id);

        let admin = Address::generate(&env);
        let oracle = Address::generate(&env);
        let manager = Address::generate(&env);

        env.mock_auths(&[
            MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "init",
                    args: (&admin,).into_val(&env),
                    sub_invocations: &[],
                },
            },
        ]);
        client.init(&admin);

        env.mock_auths(&[
            MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "grant_role",
                    args: (&admin, &oracle, Role::Oracle).into_val(&env),
                    sub_invocations: &[],
                },
            },
        ]);
        client.grant_role(&admin, &oracle, &Role::Oracle);

        env.mock_auths(&[
            MockAuth {
                address: &admin,
                invoke: &MockAuthInvoke {
                    contract: &contract_id,
                    fn_name: "grant_role",
                    args: (&admin, &manager, Role::ProjectManager).into_val(&env),
                    sub_invocations: &[],
                },
            },
        ]);
        client.grant_role(&admin, &manager, &Role::ProjectManager);

        Self {
            env,
            client,
            admin,
            oracle,
            manager,
        }
    }

    fn create_token(&self) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
        let addr = self
            .env
            .register_stellar_asset_contract_v2(self.admin.clone());
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
        let milestones = Vec::new(&self.env);
        let proof = self.dummy_proof();
        let uri = self.dummy_uri();
        
        self.env.mock_auths(&[
            MockAuth {
                address: &self.manager,
                invoke: &MockAuthInvoke {
                    contract: &self.client.address,
                    fn_name: "register_project",
                    args: (
                        &self.manager,
                        &tokens,
                        &goal,
                        &proof,
                        &uri,
                        &deadline,
                        &false,
                        &milestones,
                        &0u32,
                        &Vec::new(&self.env),
                        &0u32,
                    ).into_val(&self.env),
                    sub_invocations: &[],
                },
            },
        ]);
         let p = self.client.register_project(
             &self.manager,
             &tokens,
             &goal,
             &proof,
             &uri,
             &deadline,
             &false,
             &milestones,
             &0u32,
             &Vec::new(&self.env),
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
#[should_panic(expected = "HostError: Error(Contract, #35)")]
fn test_deposit_blocked_when_locked() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &500);

    // Simulate re-entrant state.
    ctx.force_lock();

    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.manager,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "deposit",
                args: (project_id, &ctx.manager, &token.address, 500i128).into_val(&ctx.env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &token.address,
                        fn_name: "transfer",
                        args: (&ctx.manager, &ctx.client.address, 500i128).into_val(&ctx.env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    ctx.client.deposit(&project_id, &ctx.manager, &token.address, &500i128);
}

// ── verify_and_release blocked when locked ────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #35)")]
fn test_verify_and_release_blocked_when_locked() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &1_000);
    ctx.client
        .deposit(&project_id, &ctx.manager, &token.address, &1_000i128);

    // Simulate re-entrant state.
    ctx.force_lock();

    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.oracle,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "verify_and_release",
                args: (&ctx.oracle, project_id, ctx.dummy_proof()).into_val(&ctx.env),
                sub_invocations: &[],
            },
        },
    ]);
    ctx.client.verify_and_release(&ctx.oracle, &project_id, &ctx.dummy_proof());
}

// ── refund blocked when locked ────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #35)")]
fn test_refund_blocked_when_locked() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &200);
    ctx.client
        .deposit(&project_id, &ctx.manager, &token.address, &200i128);

    // Expire the project so refund is valid.
    ctx.jump_time(86_401);
    ctx.client.expire_project(&project_id);

    // Simulate re-entrant state.
    ctx.force_lock();

    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.manager,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "refund",
                args: (&ctx.manager, project_id, &token.address).into_val(&ctx.env),
                sub_invocations: &[],
            },
        },
    ]);
    ctx.client.refund(&ctx.manager, &project_id, &token.address);
}

// ── lock is released after a normal deposit ───────────────────────────

#[test]
fn test_lock_released_after_successful_deposit() {
    let ctx = Ctx::new();
    let (token, sac) = ctx.create_token();
    let project_id = ctx.register(&token.address, 1_000);

    sac.mint(&ctx.manager, &500);
    ctx.env.mock_auths(&[
        MockAuth {
            address: &ctx.manager,
            invoke: &MockAuthInvoke {
                contract: &ctx.client.address,
                fn_name: "deposit",
                args: (project_id, &ctx.manager, &token.address, 500i128).into_val(&ctx.env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &token.address,
                        fn_name: "transfer",
                        args: (&ctx.manager, &ctx.client.address, 500i128).into_val(&ctx.env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    ctx.client.deposit(&project_id, &ctx.manager, &token.address, &500i128);

    // Lock must be cleared after the call completes.
    let contract_id = ctx.client.address.clone();
    ctx.env.as_contract(&contract_id, || {
        assert!(!crate::storage::is_locked(&ctx.env));
    });
}
