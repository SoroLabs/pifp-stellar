extern crate std;

use soroban_sdk::{testutils::Address as _, token, Address, Env};

use crate::{PifpProtocol, PifpProtocolClient, ProjectStatus, Role};

fn setup() -> (Env, PifpProtocolClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let mut ledger = env.ledger().get();
    ledger.timestamp = 100_000;
    env.ledger().set(ledger);
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    let super_admin = Address::generate(&env);
    client.init(&super_admin);
    (env, client, super_admin)
}

fn create_token(env: &Env, admin: &Address) -> token::Client<'static> {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &addr.address())
}

fn register_project(
    env: &Env,
    client: &PifpProtocolClient,
    super_admin: &Address,
    creator: &Address,
    token: &token::Client,
    deadline: u64,
    goal: i128,
) -> crate::types::Project {
    client.grant_role(super_admin, creator, &Role::ProjectManager);
    let tokens = soroban_sdk::vec![env, token.address.clone()];
    let proof = soroban_sdk::BytesN::from_array(env, &[0xabu8; 32]);
    let meta = soroban_sdk::Bytes::from_slice(env, b"bafybeigdyrzt5sfp7udm7hu76uh7y26nf3efuylqabf3oclgtqy55fbzdi");
    let empty_oracles: soroban_sdk::Vec<Address> = soroban_sdk::Vec::new(env);
    client.register_project(creator, &tokens, &goal, &proof, &meta, &deadline, &false, &empty_oracles, &0u32)
}

#[test]
fn test_refund_success_after_expiry() {
    let (env, client, super_admin) = setup();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 100;

    let project = register_project(&env, &client, &super_admin, &creator, &token, deadline, 500);

    let sac = token::StellarAssetClient::new(&env, &token.address);
    sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    let mut ledger = env.ledger().get();
    ledger.timestamp = deadline + 1;
    env.ledger().set(ledger);

    client.refund(&donator, &project.id, &token.address);

    assert_eq!(token.balance(&donator), 1_000i128);
    assert_eq!(token.balance(&client.address), 0i128);
    assert_eq!(client.get_balance(&project.id, &token.address), 0i128);
    assert_eq!(client.get_project(&project.id).status, ProjectStatus::Expired);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #21)")]
fn test_refund_fails_when_not_expired() {
    let (env, client, super_admin) = setup();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 1000;

    let project = register_project(&env, &client, &super_admin, &creator, &token, deadline, 1000);

    let sac = token::StellarAssetClient::new(&env, &token.address);
    sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);
    client.refund(&donator, &project.id, &token.address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_refund_double_refund_fails() {
    let (env, client, super_admin) = setup();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 100;

    let project = register_project(&env, &client, &super_admin, &creator, &token, deadline, 1000);

    let sac = token::StellarAssetClient::new(&env, &token.address);
    sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    let mut ledger = env.ledger().get();
    ledger.timestamp = deadline + 1;
    env.ledger().set(ledger);

    client.refund(&donator, &project.id, &token.address);
    client.refund(&donator, &project.id, &token.address);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #4)")]
fn test_refund_wrong_donator_fails() {
    let (env, client, super_admin) = setup();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let attacker = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 100;

    let project = register_project(&env, &client, &super_admin, &creator, &token, deadline, 1000);

    let sac = token::StellarAssetClient::new(&env, &token.address);
    sac.mint(&donator, &1_000i128);
    client.deposit(&project.id, &donator, &token.address, &400i128);

    let mut ledger = env.ledger().get();
    ledger.timestamp = deadline + 1;
    env.ledger().set(ledger);

    client.refund(&attacker, &project.id, &token.address);
}

#[test]
fn test_refund_success_after_cancellation() {
    let (env, client, super_admin) = setup();
    let creator = Address::generate(&env);
    let donator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 1_000;

    let project = register_project(&env, &client, &super_admin, &creator, &token, deadline, 500);

    let sac = token::StellarAssetClient::new(&env, &token.address);
    sac.mint(&donator, &700i128);
    client.deposit(&project.id, &donator, &token.address, &600i128);
    assert_eq!(client.get_project(&project.id).status, ProjectStatus::Active);

    client.cancel_project(&creator, &project.id);
    assert_eq!(client.get_project(&project.id).status, ProjectStatus::Cancelled);

    client.refund(&donator, &project.id, &token.address);
    assert_eq!(token.balance(&donator), 700i128);
    assert_eq!(token.balance(&client.address), 0i128);
}

#[test]
fn test_refund_distribution_after_cancellation_multi_donor() {
    let (env, client, super_admin) = setup();
    let creator = Address::generate(&env);
    let da = Address::generate(&env);
    let db = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let deadline = env.ledger().timestamp() + 1_000;

    let project = register_project(&env, &client, &super_admin, &creator, &token, deadline, 700);

    let sac = token::StellarAssetClient::new(&env, &token.address);
    sac.mint(&da, &1_000i128);
    sac.mint(&db, &1_000i128);
    client.deposit(&project.id, &da, &token.address, &300i128);
    client.deposit(&project.id, &db, &token.address, &500i128);
    assert_eq!(client.get_project(&project.id).status, ProjectStatus::Active);

    client.cancel_project(&super_admin, &project.id);
    client.refund(&da, &project.id, &token.address);
    client.refund(&db, &project.id, &token.address);

    assert_eq!(token.balance(&da), 1_000i128);
    assert_eq!(token.balance(&db), 1_000i128);
    assert_eq!(token.balance(&client.address), 0i128);
}
