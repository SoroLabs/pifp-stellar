extern crate std;

use crate::{test_utils::TestContext, Role};

#[test]
fn test_whitelist_funding_restricted() {
    let ctx = TestContext::new();
    let (token, _) = ctx.create_token();
    let tokens = soroban_sdk::Vec::from_array(&ctx.env, [token.address.clone()]);
    let empty_oracles: soroban_sdk::Vec<soroban_sdk::Address> = soroban_sdk::Vec::new(&ctx.env);
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &ctx.dummy_metadata_uri(),
        &(ctx.env.ledger().timestamp() + 86400),
        &true, // is_private
        &empty_oracles,
        &0u32,
    );

    let donor = ctx.generate_address();
    let sac = soroban_sdk::token::StellarAssetClient::new(&ctx.env, &token.address);
    sac.mint(&donor, &500i128);

    let result = ctx.client.try_deposit(&project.id, &donor, &token.address, &500i128);
    assert!(result.is_err());
}

#[test]
fn test_whitelist_funding_allowed() {
    let ctx = TestContext::new();
    let (token, sac) = ctx.create_token();
    let tokens = soroban_sdk::Vec::from_array(&ctx.env, [token.address.clone()]);
    let empty_oracles: soroban_sdk::Vec<soroban_sdk::Address> = soroban_sdk::Vec::new(&ctx.env);
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &ctx.dummy_metadata_uri(),
        &(ctx.env.ledger().timestamp() + 86400),
        &true,
        &empty_oracles,
        &0u32,
    );

    let donor = ctx.generate_address();
    ctx.client.add_to_whitelist(&ctx.manager, &project.id, &donor);

    sac.mint(&donor, &500i128);
    ctx.client.deposit(&project.id, &donor, &token.address, &500i128);
    assert_eq!(ctx.client.get_balance(&project.id, &token.address), 500i128);
}

#[test]
fn test_whitelist_management_auth() {
    let ctx = TestContext::new();
    let (token, _) = ctx.create_token();
    let tokens = soroban_sdk::Vec::from_array(&ctx.env, [token.address.clone()]);
    let empty_oracles: soroban_sdk::Vec<soroban_sdk::Address> = soroban_sdk::Vec::new(&ctx.env);
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &ctx.dummy_metadata_uri(),
        &(ctx.env.ledger().timestamp() + 86400),
        &true,
        &empty_oracles,
        &0u32,
    );

    let stranger = ctx.generate_address();
    let donor = ctx.generate_address();

    // Stranger cannot add to whitelist.
    let result = ctx.client.try_add_to_whitelist(&stranger, &project.id, &donor);
    assert!(result.is_err());

    // Admin can add.
    ctx.client.add_to_whitelist(&ctx.admin, &project.id, &donor);

    // Creator can remove.
    ctx.client.remove_from_whitelist(&ctx.manager, &project.id, &donor);
}
