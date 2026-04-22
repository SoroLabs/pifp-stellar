use crate::test_utils::{create_token, dummy_metadata_uri, dummy_proof, setup_test};
use crate::Role;
use soroban_sdk::{
    testutils::{Address as _, MockAuth, MockAuthInvoke},
    token, Address, Vec, IntoVal
};

#[test]
fn test_whitelist_funding_restricted() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    env.mock_auths(&[
        MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "grant_role",
                args: (&admin, &creator, Role::ProjectManager).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    client.grant_role(&admin, &creator, &Role::ProjectManager);

    // Register a private project
    let milestones = Vec::new(&env);
    let proof_hash = dummy_proof(&env);
    env.mock_auths(&[
        MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "register_project",
                args: (
                    &creator,
                    &accepted_tokens,
                    1000i128,
                    &proof_hash,
                    dummy_metadata_uri(&env),
                    env.ledger().timestamp() + 10000,
                    true,
                    &milestones,
                    0u32,
                    Vec::new(&env),
                    0u32,
                ).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &proof_hash,
        &dummy_metadata_uri(&env),
        &(env.ledger().timestamp() + 10000),
        &true, // is_private
        &milestones,
        &0u32,
        &Vec::new(&env),
        &0u32,
    );

    // Attempt deposit from non-whitelisted donor
    token_sac.mint(&donor, &500);
    env.mock_auths(&[
        MockAuth {
            address: &donor,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "deposit",
                args: (project.id, &donor, &token.address, 500i128).into_val(&env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &token.address,
                        fn_name: "transfer",
                        args: (&donor, &client.address, 500i128).into_val(&env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    let result = client.try_deposit(&project.id, &donor, &token.address, &500);

    assert!(result.is_err());
}

#[test]
fn test_whitelist_funding_allowed() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &(env.ledger().timestamp() + 10000),
        &true,
        &0u32,
    );

    // Add donor to whitelist
    env.mock_auths(&[
        MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "add_to_whitelist",
                args: (&creator, project.id, &donor).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    client.add_to_whitelist(&creator, &project.id, &donor);

    // Deposit should now work
    token_sac.mint(&donor, &500);
    env.mock_auths(&[
        MockAuth {
            address: &donor,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "deposit",
                args: (project.id, &donor, &token.address, 500i128).into_val(&env),
                sub_invocations: &[
                    MockAuthInvoke {
                        contract: &token.address,
                        fn_name: "transfer",
                        args: (&donor, &client.address, 500i128).into_val(&env),
                        sub_invocations: &[],
                    }
                ],
            },
        },
    ]);
    client.deposit(&project.id, &donor, &token.address, &500);

    let balance = client.get_balance(&project.id, &token.address);
    assert_eq!(balance, 500);
}

#[test]
fn test_whitelist_management_auth() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let stranger = Address::generate(&env);
    let donor = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &(env.ledger().timestamp() + 10000),
        &true,
        &0u32,
    );

    // Stranger cannot add to whitelist
    env.mock_auths(&[
        MockAuth {
            address: &stranger,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "add_to_whitelist",
                args: (&stranger, project.id, &donor).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    let result = client.try_add_to_whitelist(&stranger, &project.id, &donor);
    assert!(result.is_err());

    // Admin CAN add to whitelist
    env.mock_auths(&[
        MockAuth {
            address: &admin,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "add_to_whitelist",
                args: (&admin, project.id, &donor).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    client.add_to_whitelist(&admin, &project.id, &donor);

    // Creator can remove
    env.mock_auths(&[
        MockAuth {
            address: &creator,
            invoke: &MockAuthInvoke {
                contract: &client.address,
                fn_name: "remove_from_whitelist",
                args: (&creator, project.id, &donor).into_val(&env),
                sub_invocations: &[],
            },
        },
    ]);
    client.remove_from_whitelist(&creator, &project.id, &donor);
}
