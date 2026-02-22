extern crate std;

use soroban_sdk::{
    testutils::Address as _,
    token, Address, BytesN, Env,
};

use crate::{PifpProtocol, PifpProtocolClient, Role};

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    (env, client)
}

fn setup_with_init() -> (Env, PifpProtocolClient<'static>, Address) {
    let (env, client) = setup();
    let super_admin = Address::generate(&env);
    client.init(&super_admin);
    (env, client, super_admin)
}

fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &addr.address())
}

fn dummy_proof(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xabu8; 32])
}

fn future_deadline(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400
}

#[test]
fn test_donation_count_initialized_to_zero() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    assert_eq!(project.donation_count, 0);
}

#[test]
fn test_donation_count_increments_for_new_donor() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let donator = Address::generate(&env);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    // Mint tokens to donator
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &1_000i128);

    // First deposit
    client.deposit(&project.id, &donator, &token.address, &500i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 1);
}

#[test]
fn test_donation_count_stays_same_for_repeated_donor() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let donator = Address::generate(&env);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    // Mint tokens to donator
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &2_000i128);

    // First deposit
    client.deposit(&project.id, &donator, &token.address, &500i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 1);

    // Second deposit from same donor with same token
    client.deposit(&project.id, &donator, &token.address, &300i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 1); // Should still be 1
}

#[test]
fn test_donation_count_increments_for_different_donors() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let donator1 = Address::generate(&env);
    let donator2 = Address::generate(&env);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    // Mint tokens to both donators
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator1, &1_000i128);
    token_sac.mint(&donator2, &1_000i128);

    // First donor deposits
    client.deposit(&project.id, &donator1, &token.address, &500i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 1);

    // Second donor deposits
    client.deposit(&project.id, &donator2, &token.address, &300i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 2);
}

#[test]
fn test_donation_count_increments_for_same_donor_different_tokens() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token1 = create_token(&env, &token_admin);
    let token2 = create_token(&env, &token_admin);
    let donator = Address::generate(&env);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token1.address.clone(), token2.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    // Mint both tokens to donator
    let token1_sac = token::StellarAssetClient::new(&env, &token1.address);
    let token2_sac = token::StellarAssetClient::new(&env, &token2.address);
    token1_sac.mint(&donator, &1_000i128);
    token2_sac.mint(&donator, &1_000i128);

    // Deposit with first token
    client.deposit(&project.id, &donator, &token1.address, &500i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 1);

    // Deposit with second token (same donor, different token)
    client.deposit(&project.id, &donator, &token2.address, &300i128);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.donation_count, 2); // Should increment to 2
}

#[test]
fn test_donation_count_complex_scenario() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token1 = create_token(&env, &token_admin);
    let token2 = create_token(&env, &token_admin);
    let donator1 = Address::generate(&env);
    let donator2 = Address::generate(&env);
    let donator3 = Address::generate(&env);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token1.address.clone(), token2.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    // Mint tokens to all donators
    let token1_sac = token::StellarAssetClient::new(&env, &token1.address);
    let token2_sac = token::StellarAssetClient::new(&env, &token2.address);
    token1_sac.mint(&donator1, &5_000i128);
    token1_sac.mint(&donator2, &5_000i128);
    token1_sac.mint(&donator3, &5_000i128);
    token2_sac.mint(&donator1, &5_000i128);
    token2_sac.mint(&donator2, &5_000i128);

    // donator1 deposits with token1
    client.deposit(&project.id, &donator1, &token1.address, &100i128);
    assert_eq!(client.get_project(&project.id).donation_count, 1);

    // donator1 deposits again with token1 (should not increment)
    client.deposit(&project.id, &donator1, &token1.address, &100i128);
    assert_eq!(client.get_project(&project.id).donation_count, 1);

    // donator2 deposits with token1 (new donor-token pair)
    client.deposit(&project.id, &donator2, &token1.address, &200i128);
    assert_eq!(client.get_project(&project.id).donation_count, 2);

    // donator1 deposits with token2 (same donor, different token)
    client.deposit(&project.id, &donator1, &token2.address, &150i128);
    assert_eq!(client.get_project(&project.id).donation_count, 3);

    // donator3 deposits with token1 (new donor)
    client.deposit(&project.id, &donator3, &token1.address, &300i128);
    assert_eq!(client.get_project(&project.id).donation_count, 4);

    // donator2 deposits with token2 (new donor-token pair)
    client.deposit(&project.id, &donator2, &token2.address, &250i128);
    assert_eq!(client.get_project(&project.id).donation_count, 5);

    // donator2 deposits again with token2 (should not increment)
    client.deposit(&project.id, &donator2, &token2.address, &100i128);
    assert_eq!(client.get_project(&project.id).donation_count, 5);
}
