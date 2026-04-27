extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger, MockAuth, MockAuthInvoke},
    token, Address, Bytes, BytesN, Env, IntoVal, Val, Vec,
};

use crate::{DepositRequest, PifpProtocol, PifpProtocolClient, Role};

fn setup() -> (Env, PifpProtocolClient<'static>, Address, Address, Address) {
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

    (env, client, admin, oracle, manager)
}

fn create_token(
    env: &Env,
    admin: &Address,
) -> (token::Client<'static>, token::StellarAssetClient<'static>) {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    (
        token::Client::new(env, &addr.address()),
        token::StellarAssetClient::new(env, &addr.address()),
    )
}

fn register(
    env: &Env,
    client: &PifpProtocolClient,
    manager: &Address,
    token_addr: &Address,
    goal: i128,
) -> u64 {
    let tokens = soroban_sdk::vec![env, token_addr.clone()];
    let deadline = env.ledger().timestamp() + 86_400;
    let proof = BytesN::from_array(env, &[0xabu8; 32]);
    let uri = Bytes::from_slice(env, b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");

    let milestones = Vec::new(env);
    env.mock_auths(&[
        MockAuth {
            address: manager,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "register_project",
                args: (
                    manager,
                    &tokens,
                    &goal,
                    &proof,
                    &uri,
                    &deadline,
                    &false,
                    &milestones,
                    &0u32, // categories
                    &Vec::new(env), // authorized_oracles
                    &0u32, // threshold
                ).into_val(env),
                sub_invocations: &[],
            },
        },
    ]);
    client.register_project(manager, &tokens, &goal, &proof, &uri, &deadline, &false, &milestones, &0u32, &Vec::new(env), &0u32).id

#[test]
fn test_batch_deposit_funds_multiple_projects() {
    let (env, client, admin, _oracle, manager) = setup();
    let donator = Address::generate(&env);

    let (tok1, sac1) = create_token(&env, &admin);
    let (tok2, sac2) = create_token(&env, &admin);

    let pid1 = register(&env, &client, &manager, &tok1.address, 1_000);
    let pid2 = register(&env, &client, &manager, &tok2.address, 2_000);

    sac1.mint(&donator, &500);
    sac2.mint(&donator, &800);

    let deposits = soroban_sdk::vec![
        &env,
        DepositRequest {
            project_id: pid1,
            token: tok1.address.clone(),
            amount: 500
        },
        DepositRequest {
            project_id: pid2,
            token: tok2.address.clone(),
            amount: 800
        },
    ];

    env.mock_auths(&[
        MockAuth {
            address: &donator,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "batch_deposit",
                args: (&donator, &deposits).into_val(&env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &tok1.address,
                        fn_name: "transfer",
                        args: (&donator, &client.address, 500i128).into_val(&env),
                        sub_invocations: &[],
                    },
                    MockAuthInvoke {
                        contract: &tok2.address,
                        fn_name: "transfer",
                        args: (&donator, &client.address, 800i128).into_val(&env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    client.batch_deposit(&donator, &deposits);

    assert_eq!(client.get_balance(&pid1, &tok1.address), 500);
    assert_eq!(client.get_balance(&pid2, &tok2.address), 800);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #11)")]
fn test_batch_deposit_reverts_on_invalid_amount() {
    let (env, client, admin, _oracle, manager) = setup();
    let donator = Address::generate(&env);

    let (tok1, sac1) = create_token(&env, &admin);
    let (tok2, _sac2) = create_token(&env, &admin);

    let pid1 = register(&env, &client, &manager, &tok1.address, 1_000);
    let pid2 = register(&env, &client, &manager, &tok2.address, 1_000);

    sac1.mint(&donator, &500);

    // Second entry has amount=0 — should panic and revert the whole tx.
    let deposits = soroban_sdk::vec![
        &env,
        DepositRequest {
            project_id: pid1,
            token: tok1.address.clone(),
            amount: 500
        },
        DepositRequest {
            project_id: pid2,
            token: tok2.address.clone(),
            amount: 0
        },
    ];

    env.mock_auths(&[
        MockAuth {
            address: &donator,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "batch_deposit",
                args: (&donator, &deposits).into_val(&env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &tok1.address,
                        fn_name: "transfer",
                        args: (&donator, &client.address, 500i128).into_val(&env),
                        sub_invocations: &[],
                    },
                    MockAuthInvoke {
                        contract: &tok2.address,
                        fn_name: "transfer",
                        args: (&donator, &client.address, 0i128).into_val(&env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    client.batch_deposit(&donator, &deposits);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #19)")]
fn test_batch_deposit_blocked_when_paused() {
    let (env, client, admin, _oracle, manager) = setup();
    let donator = Address::generate(&env);
    let (tok1, sac1) = create_token(&env, &admin);
    let pid1 = register(&env, &client, &manager, &tok1.address, 1_000);

    sac1.mint(&donator, &500);
    env.mock_auths(&[
        MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "pause",
                args: (&admin,).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    client.pause(&admin);

    let deposits = soroban_sdk::vec![
        &env,
        DepositRequest {
            project_id: pid1,
            token: tok1.address.clone(),
            amount: 500
        },
    ];
    env.mock_auths(&[
        MockAuth {
            address: &donator,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "batch_deposit",
                args: (&donator, &deposits).into_val(&env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &tok1.address,
                        fn_name: "transfer",
                        args: (&donator, &client.address, 500i128).into_val(&env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    client.batch_deposit(&donator, &deposits);
}
