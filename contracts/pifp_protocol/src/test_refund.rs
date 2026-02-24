extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, BytesN, Env,
};

use crate::{PifpProtocol, PifpProtocolClient, ProjectStatus, Role};

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    env.ledger().set(LedgerInfo {
        timestamp: 100_000,
        protocol_version: 22,
        sequence_number: 100,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 1000,
    });
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

#[test]
fn test_refund_success_after_expiry() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 100;

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project =
        client.register_project(&creator, &tokens, &1_000i128, &dummy_proof(&env), &deadline);

    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 22,
        sequence_number: 101,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 1000,
    });

    client.refund(&donator, &project.id, &token.address);

    let token_client = token::Client::new(&env, &token.address);
    assert_eq!(token_client.balance(&donator), 1_000i128);
    assert_eq!(token_client.balance(&client.address), 0i128);
    assert_eq!(client.get_balance(&project.id, &token.address), 0i128);
    assert_eq!(
        client.get_project(&project.id).status,
        ProjectStatus::Expired
    );

    let contract_id = client.address.clone();
    env.as_contract(&contract_id, || {
        assert_eq!(
            crate::storage::get_donator_balance(&env, project.id, &token.address, &donator),
            0
        );
    });
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #21)")]
fn test_refund_fails_when_not_expired() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 1000;

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project =
        client.register_project(&creator, &tokens, &1_000i128, &dummy_proof(&env), &deadline);

    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    client.refund(&donator, &project.id, &token.address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_refund_double_refund_fails() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 100;

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project =
        client.register_project(&creator, &tokens, &1_000i128, &dummy_proof(&env), &deadline);

    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 22,
        sequence_number: 101,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 1000,
    });

    client.refund(&donator, &project.id, &token.address);
    client.refund(&donator, &project.id, &token.address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_refund_wrong_donator_fails() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 100;

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project =
        client.register_project(&creator, &tokens, &1_000i128, &dummy_proof(&env), &deadline);

    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    env.ledger().set(LedgerInfo {
        timestamp: deadline + 1,
        protocol_version: 22,
        sequence_number: 101,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 1000,
    });

    client.refund(&attacker, &project.id, &token.address);
}
