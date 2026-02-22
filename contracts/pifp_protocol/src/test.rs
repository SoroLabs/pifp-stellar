extern crate std;

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, BytesN, Env, Vec,
};

use crate::{PifpProtocol, PifpProtocolClient, Role, ProjectStatus};

// ─── Helpers ─────────────────────────────────────────────

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    
    // Initialize ledger with a non-zero timestamp
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
    let (_env, client, super_admin) = setup_with_init();
    assert!(client.has_role(&super_admin, &Role::SuperAdmin));
    assert_eq!(client.role_of(&super_admin), Some(Role::SuperAdmin));
}

#[test]
#[should_panic]
fn test_init_twice_panics() {
    let (_env, client, super_admin) = setup_with_init();
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

// ─── 3. Security Hardening Tests ─────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_register_duplicate_tokens_fails() {
    let (env, client, admin) = setup_with_init();
    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone(), token.clone()]);
    
    client.register_project(&admin, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_register_zero_goal_fails() {
    let (env, client, admin) = setup_with_init();
    let tokens = Vec::from_array(&env, [Address::generate(&env)]);
    
    client.register_project(&admin, &tokens, &0i128, &dummy_proof(&env), &future_deadline(&env));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_register_past_deadline_fails() {
    let (env, client, admin) = setup_with_init();
    let tokens = Vec::from_array(&env, [Address::generate(&env)]);
    let past_deadline = env.ledger().timestamp() - 1;
    
    client.register_project(&admin, &tokens, &1000i128, &dummy_proof(&env), &past_deadline);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #11)")]
fn test_deposit_zero_amount_fails() {
    let (env, client, admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone()]);
    
    client.grant_role(&admin, &creator, &Role::ProjectManager);
    let project = client.register_project(&creator, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
    
    client.deposit(&project.id, &creator, &token, &0i128);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_deposit_after_deadline_fails() {
    let (env, client, admin) = setup_with_init();
    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone()]);
    
    let pm = Address::generate(&env);
    client.grant_role(&admin, &pm, &Role::ProjectManager);
    let project = client.register_project(&pm, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
    
    // Fast-forward time
    env.ledger().set(LedgerInfo {
        timestamp: future_deadline(&env) + 1,
        protocol_version: 22,
        sequence_number: 100,
        network_id: [0u8; 32],
        base_reserve: 10,
        min_temp_entry_ttl: 10,
        min_persistent_entry_ttl: 10,
        max_entry_ttl: 1000,
    });
    
    client.deposit(&project.id, &admin, &token, &100i128);
}

// ─── 4. Emergency Pause Tests ────────────────────────────

#[test]
fn test_admin_can_pause_and_unpause() {
    let (env, client, admin) = setup_with_init();
    
    assert!(!client.is_paused());
    
    client.pause(&admin);
    assert!(client.is_paused());
    
    client.unpause(&admin);
    assert!(!client.is_paused());
}

// ─── 9. Storage retrieval optimisations ─────────────────────

#[test]
fn test_project_exists_and_maybe_load_helpers() {
    let (env, client, super_admin) = setup_with_init();
    let contract_id = client.address.clone();
    let token = Address::generate(&env);

    // nothing registered yet
    env.as_contract(&contract_id, || {
        assert!(!crate::storage::project_exists(&env, 0));
        assert_eq!(crate::storage::maybe_load_project(&env, 0), None);
        assert_eq!(crate::storage::maybe_load_project_config(&env, 0), None);
        assert_eq!(crate::storage::maybe_load_project_state(&env, 0), None);
    });

    // register one project and exercise the new helpers
    let pm = Address::generate(&env);
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    let tokens = Vec::from_array(&env, [token.clone()]);
    let project = client.register_project(
        &pm,
        &tokens,
        &1_000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    env.as_contract(&contract_id, || {
        assert!(crate::storage::project_exists(&env, project.id));
        // individual maybe_load functions should return some value matching fields
        let cfg = crate::storage::maybe_load_project_config(&env, project.id)
            .expect("config should exist");
        assert_eq!(cfg.id, project.id);
        let st = crate::storage::maybe_load_project_state(&env, project.id)
            .expect("state should exist");
        assert_eq!(st.donation_count, 0);

        // maybe_load_project returns full struct
        let loaded = crate::storage::maybe_load_project(&env, project.id)
            .expect("project exists");
        assert_eq!(loaded.creator, project.creator);

        // load_project_pair should match load_project
        let (cfg2, st2) = crate::storage::load_project_pair(&env, project.id);
        let full = crate::storage::load_project(&env, project.id);
        assert_eq!(full.creator, cfg2.creator);
        assert_eq!(full.donation_count, st2.donation_count);
    });
}

#[test]
#[should_panic]
fn test_load_project_pair_panics_for_missing() {
    let (env, _client, _super_admin) = setup_with_init();
    // id 42 not present -> should panic
    crate::storage::load_project_pair(&env, 42);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_non_admin_cannot_pause() {
    let (env, client, _admin) = setup_with_init();
    let rando = Address::generate(&env);
    
    client.pause(&rando);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #19)")]
fn test_registration_fails_when_paused() {
    let (env, client, admin) = setup_with_init();
    client.pause(&admin);
    
    let tokens = Vec::from_array(&env, [Address::generate(&env)]);
    client.register_project(&admin, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #19)")]
fn test_deposit_fails_when_paused() {
    let (env, client, admin) = setup_with_init();
    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone()]);
    
    let pm = Address::generate(&env);
    client.grant_role(&admin, &pm, &Role::ProjectManager);
    let project = client.register_project(&pm, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
    
    client.pause(&admin);
    client.deposit(&project.id, &pm, &token, &100i128);
}

#[test]
fn test_queries_work_when_paused() {
    let (env, client, admin) = setup_with_init();
    let tokens = Vec::from_array(&env, [Address::generate(&env)]);
    
    let pm = Address::generate(&env);
    client.grant_role(&admin, &pm, &Role::ProjectManager);
    let project = client.register_project(&pm, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
    
    client.pause(&admin);
    
    // Query should still work
    let loaded = client.get_project(&project.id);
    assert_eq!(loaded.id, project.id);
}