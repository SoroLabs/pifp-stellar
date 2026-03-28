#![cfg(test)]

use crate::test_utils::{create_token, setup_test};
use crate::{Error, ProjectStatus, Role};
use soroban_sdk::{testutils::{Address as _, Ledger}, Address, Vec};

#[test]
fn test_update_protocol_config_success() {
    let (env, client, admin) = setup_test();
    let recipient = Address::generate(&env);
    
    // Init sets admin as SuperAdmin
    client.update_protocol_config(&admin, &recipient, &500); // 5%
    
    // No direct getter, but we can verify it works by running a release
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_update_protocol_config_unauthorized() {
    let (env, client, _admin) = setup_test();
    let stranger = Address::generate(&env);
    let recipient = Address::generate(&env);
    
    client.update_protocol_config(&stranger, &recipient, &500);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #25)")]
fn test_update_protocol_config_invalid_bps() {
    let (env, client, admin) = setup_test();
    let recipient = Address::generate(&env);
    
    client.update_protocol_config(&admin, &recipient, &1001); // > 10%
}

#[test]
fn test_verify_and_release_with_fees() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let oracle = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);
    
    // Setup roles
    client.grant_role(&admin, &creator, &Role::ProjectManager);
    client.grant_role(&admin, &oracle, &Role::Oracle);
    
    // Set 5% fee
    client.update_protocol_config(&admin, &fee_recipient, &500);
    
    let proof_hash = [1u8; 32].into();
    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &proof_hash,
        &(env.ledger().timestamp() + 10000),
    );
    
    // Deposit 1000 tokens
    token.mint(&donor, &1000);
    client.deposit(&project.id, &donor, &token.address, &1000);
    
    // Verify and release
    client.verify_and_release(&oracle, &project.id, &proof_hash);
    
    // Fee = 1000 * 500 / 10000 = 50 tokens
    // Creator = 1000 - 50 = 950 tokens
    
    assert_eq!(token.balance(&fee_recipient), 50);
    assert_eq!(token.balance(&creator), 950);
    assert_eq!(token.balance(&client.address), 0);
}

#[test]
fn test_verify_and_release_zero_fee() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let donor = Address::generate(&env);
    let oracle = Address::generate(&env);
    let fee_recipient = Address::generate(&env);
    
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);
    
    client.grant_role(&admin, &creator, &Role::ProjectManager);
    client.grant_role(&admin, &oracle, &Role::Oracle);
    
    // Set 0% fee
    client.update_protocol_config(&admin, &fee_recipient, &0);
    
    let proof_hash = [1u8; 32].into();
    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &proof_hash,
        &(env.ledger().timestamp() + 10000),
    );
    
    token.mint(&donor, &1000);
    client.deposit(&project.id, &donor, &token.address, &1000);
    
    client.verify_and_release(&oracle, &project.id, &proof_hash);
    
    assert_eq!(token.balance(&fee_recipient), 0);
    assert_eq!(token.balance(&creator), 1000);
}
