extern crate std;

use soroban_sdk::{
    testutils::Address as _,
    token, Address, BytesN, Env, Vec,
};

use crate::{PifpProtocol, PifpProtocolClient, Role, ProjectStatus};

// ─── Helpers ─────────────────────────────────────────────

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register_contract(None, PifpProtocol);
    let client = PifpProtocolClient::new(&env, &contract_id);
    (env, client)
}

fn setup_with_init() -> (Env, PifpProtocolClient<'static>, Address) {
    let (env, client) = setup();
    let super_admin = Address::generate(&env);
    client.init(&super_admin);
    (env, client, super_admin)
}

fn create_token_contract<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &addr.address())
}

fn dummy_proof(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xabu8; 32])
}

fn future_deadline(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400
}

// ─── 1. Initialisation ───────────────────────────────────

#[test]
fn test_init_sets_super_admin() {
    let (env, client, super_admin) = setup_with_init();
    assert!(client.has_role(&super_admin, &Role::SuperAdmin));
    assert_eq!(client.role_of(&super_admin), Some(Role::SuperAdmin));
}

#[test]
#[should_panic]
fn test_init_twice_panics() {
    let (env, client, super_admin) = setup_with_init();
    client.init(&super_admin);
}

// ─── 2. register_project ─────────────────────────────────

#[test]
fn test_register_project_success() {
    let (env, client, super_admin) = setup_with_init();
    
    let _creator = Address::generate(&env);
    let token = Address::generate(&env);
    let mut tokens = Vec::new(&env);
    tokens.push_back(token.clone());
    
    let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
    let goal: i128 = 1_000;
    let deadline = future_deadline(&env);

    let project = client.register_project(&super_admin, &tokens, &goal, &proof_hash, &deadline);

    assert_eq!(project.id, 0);
    assert_eq!(project.creator, super_admin);
    assert_eq!(project.accepted_tokens.get(0).unwrap(), token);
    assert_eq!(project.goal, goal);
    assert_eq!(project.proof_hash, proof_hash);
    assert_eq!(project.deadline, deadline);
    assert_eq!(project.status, ProjectStatus::Funding);
}

// Note: Many other tests are in rbac_test.rs and test_events.rs.
// This file serves as a basic integration verification.