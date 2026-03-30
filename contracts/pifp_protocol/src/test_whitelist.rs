use crate::test_utils::{create_token, dummy_metadata_uri, dummy_proof, setup_test};
use crate::Role;
use soroban_sdk::{testutils::Address as _, token, Address, Vec};

#[test]
fn test_whitelist_funding_restricted() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let token = create_token(&env, &admin);
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    // Register a private project
    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &(env.ledger().timestamp() + 10000),
        &true, // is_private
        &0u32,
    );

    // Attempt deposit from non-whitelisted donor
    token_sac.mint(&donor, &500);
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
    client.add_to_whitelist(&creator, &project.id, &donor);

    // Deposit should now work
    token_sac.mint(&donor, &500);
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
    let result = client.try_add_to_whitelist(&stranger, &project.id, &donor);
    assert!(result.is_err());

    // Admin CAN add to whitelist
    client.add_to_whitelist(&admin, &project.id, &donor);

    // Creator can remove
    client.remove_from_whitelist(&creator, &project.id, &donor);
}
