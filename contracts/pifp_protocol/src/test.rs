#![cfg(test)]

extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    token, Address, BytesN, Env, IntoVal, String,
};

use crate::types::ProjectStatus;
use crate::{PifpProtocol, PifpProtocolClient};

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    (env, client)
}

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let contract_address = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &contract_address.address())
}

// ── Project Registry Tests ──────────────────────────────────────────

#[test]
fn test_register_project_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
    let goal: i128 = 1_000;
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token.address, &goal, &proof_hash, &deadline);

    assert_eq!(project.id, 0);
    assert_eq!(project.creator, creator);
    assert_eq!(project.token, token.address);
    assert_eq!(project.goal, goal);
    assert_eq!(project.balance, 0);
    assert_eq!(project.proof_hash, proof_hash);
    assert_eq!(project.deadline, deadline);
    assert_eq!(project.status, ProjectStatus::Funding);
}

#[test]
fn test_register_second_project_unique_ids() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[2u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let p1 = client.register_project(&creator, &token.address, &500, &proof_hash, &deadline);
    let p2 = client.register_project(&creator, &token.address, &700, &proof_hash, &deadline);

    assert_eq!(p1.id, 0);
    assert_eq!(p2.id, 1);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #5)")]
fn test_register_project_invalid_goal() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[3u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    client.register_project(&creator, &token.address, &0, &proof_hash, &deadline);
}

#[test]
#[should_panic(expected = "deadline must be in the future")]
fn test_register_project_invalid_deadline() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[4u8; 32]);
    let deadline: u64 = 0;

    client.register_project(&creator, &token.address, &100, &proof_hash, &deadline);
}

#[test]
fn test_get_project_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[5u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let registered = client.register_project(&creator, &token.address, &999, &proof_hash, &deadline);
    let retrieved = client.get_project(&registered.id);

    assert_eq!(registered, retrieved);
}

#[test]
#[should_panic(expected = "project not found")]
fn test_get_project_not_found() {
    let (_env, client) = setup();

    client.get_project(&42);
}

// ── ZK-Proof Verification Tests ─────────────────────────────────────

#[test]
fn test_set_oracle() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.set_oracle(&admin, &oracle);
}

#[test]
fn test_verify_and_release_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token.address, &500, &proof_hash, &deadline);

    let oracle_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&oracle_admin, &oracle);

    // Oracle verifies with the correct proof hash.
    client.verify_and_release(&project.id, &proof_hash);

    // Check project status is now Completed.
    let updated = client.get_project(&project.id);
    assert_eq!(updated.status, ProjectStatus::Completed);
}

#[test]
#[should_panic(expected = "proof verification failed: hash mismatch")]
fn test_verify_wrong_hash() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let wrong_hash = BytesN::from_array(&env, &[99u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token.address, &500, &proof_hash, &deadline);

    let oracle_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&oracle_admin, &oracle);

    client.verify_and_release(&project.id, &wrong_hash);
}

#[test]
#[should_panic(expected = "project already completed")]
fn test_verify_already_completed() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token.address, &500, &proof_hash, &deadline);

    let oracle_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&oracle_admin, &oracle);

    // First verification succeeds.
    client.verify_and_release(&project.id, &proof_hash);

    // Second verification should fail.
    client.verify_and_release(&project.id, &proof_hash);
}

#[test]
#[should_panic(expected = "project not found")]
fn test_verify_nonexistent_project() {
    let (env, client) = setup();

    let oracle_admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&oracle_admin, &oracle);

    let fake_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.verify_and_release(&999, &fake_hash);
}

#[test]
#[should_panic(expected = "oracle not set")]
fn test_verify_without_oracle_set() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let admin = Address::generate(&env);
    let token = create_token_contract(&env, &admin);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token.address, &500, &proof_hash, &deadline);

    // No oracle set — should panic.
    client.verify_and_release(&project.id, &proof_hash);
}

// ── Deposit Tests ───────────────────────────────────────────────────

#[test]
fn test_deposit_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let mock_token_client = create_token_contract(&env, &token_admin);
    let token = mock_token_client.address.clone();
    
    let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
    let goal: i128 = 1_000;
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token, &goal, &proof_hash, &deadline);

    let donator = Address::generate(&env);
    
    // Mint tokens to donator
    let token_admin_client = token::StellarAssetClient::new(&env, &token);
    token_admin_client.mint(&donator, &500);
    
    // Verify starting balance
    assert_eq!(mock_token_client.balance(&donator), 500);

    // Deposit
    client.deposit(&project.id, &donator, &500);

    // Verify balances
    assert_eq!(mock_token_client.balance(&donator), 0);
    assert_eq!(mock_token_client.balance(&client.address), 500);

    // Verify project state
    let updated = client.get_project(&project.id);
    assert_eq!(updated.balance, 500);
}

#[test]
#[should_panic(expected = "project not found")]
fn test_deposit_project_not_found() {
    let (env, client) = setup();

    let donator = Address::generate(&env);
    client.deposit(&999, &donator, &500);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #10)")]
fn test_deposit_insufficient_balance() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let mock_token_client = create_token_contract(&env, &token_admin);
    let token = mock_token_client.address.clone();
    
    let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
    let goal: i128 = 1_000;
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &token, &goal, &proof_hash, &deadline);

    let donator = Address::generate(&env);
    // Donator has 0 balance, so deposit should panic.
    client.deposit(&project.id, &donator, &500);
}
