extern crate std;

 error_handling
extern crate alloc;

use soroban_sdk::{
    testutils::{Address as _, Ledger, LedgerInfo},
    token, Address, BytesN, Env, Vec, InvokeError,
};
use alloc::string::String;
use core::fmt::Write as _;

use crate::{PifpProtocol, PifpProtocolClient, Role, ProjectStatus, Error};

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

#[allow(dead_code)]
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

fn assert_contract_err<T: core::fmt::Debug>(res: T, expected: Error) {
    let mut s = String::new();
    let _ = write!(s, "{:?}", res);
    let mut code_str = String::new();
    let _ = write!(code_str, "Contract({})", expected as u32);
    let mut enum_str = String::new();
    let _ = write!(enum_str, "{:?}", expected);
    let mut hash_str = String::new();
    let _ = write!(hash_str, "#{}", expected as u32);

    if s.contains(&code_str) || s.contains(&enum_str) || s.contains(&hash_str) {
        return;
    }
    panic!("expected contract error {:?}, got {:?}", expected, s);
}


// ─── 1. Initialisation ───────────────────────────────────

use crate::{test_utils::TestContext, ProjectStatus, Role};
use soroban_sdk::Vec;
 main

#[test]
fn test_init_sets_super_admin() {
    let ctx = TestContext::new();
    assert!(ctx.client.has_role(&ctx.admin, &Role::SuperAdmin));
    assert_eq!(ctx.client.role_of(&ctx.admin), Some(Role::SuperAdmin));
}

#[test]
#[should_panic]
fn test_init_twice_panics() {
    let ctx = TestContext::new();
    ctx.client.init(&ctx.admin);
}

#[test]
fn test_register_project_success() {
    let ctx = TestContext::new();
    let token = ctx.generate_address();
    let tokens = Vec::from_array(&ctx.env, [token.clone()]);
    let goal: i128 = 1_000;

    let project = ctx.register_project(&tokens, goal);

    assert_eq!(project.id, 0);
    assert_eq!(project.creator, ctx.manager);
    assert_eq!(project.accepted_tokens.get(0).unwrap(), token);
    assert_eq!(project.goal, goal);
    assert_eq!(project.status, ProjectStatus::Funding);
}

#[test]
 error_handling
fn test_non_admin_cannot_pause() {
    let (env, client, _admin) = setup_with_init();
    let rando = Address::generate(&env);

    let result = client.try_pause(&rando);

    assert_contract_err(result, Error::NotAuthorized);
}



#[test]
fn test_deposit_token_not_accepted_fails() {
    let (env, client, admin) = setup_with_init();

    let valid_token = Address::generate(&env);
    let invalid_token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [valid_token.clone()]);

    let pm = Address::generate(&env);
    client.grant_role(&admin, &pm, &Role::ProjectManager);

    let project = client.register_project(
        &pm,
        &tokens,
        &1000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    let result = client.try_deposit(&project.id, &pm, &invalid_token, &100i128);

    assert_contract_err(result, Error::TokenNotAccepted);

#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_register_duplicate_tokens_fails() {
    let ctx = TestContext::new();
    let token = ctx.generate_address();
    let tokens = Vec::from_array(&ctx.env, [token.clone(), token.clone()]);

    ctx.register_project(&tokens, 1000);
 main
}



#[test]
fn test_register_zero_goal_fails() {
 error_handling
    let (env, client, admin) = setup_with_init();
    let tokens = Vec::from_array(&env, [Address::generate(&env)]);

    let result = client.try_register_project(
        &admin,
        &tokens,
        &0i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    assert_contract_err(result, Error::InvalidGoal);

    let ctx = TestContext::new();
    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);
    ctx.register_project(&tokens, 0);
 main
}




#[test]
fn test_register_past_deadline_fails() {
 error_handling
    let (env, client, admin) = setup_with_init();
    let tokens = Vec::from_array(&env, [Address::generate(&env)]);
    let past_deadline = env.ledger().timestamp() - 1;

    let result = client.try_register_project(
        &admin,
        &tokens,
        &1000i128,
        &dummy_proof(&env),
        &past_deadline,
    );

    assert_contract_err(result, Error::InvalidDeadline);

    let ctx = TestContext::new();
    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);

    // Set ledger to future
    ctx.jump_time(200_000);

    // Attempt to register with a past deadline (86400 from 100_000 < 200_000)
    let past_deadline = 150_000;
    ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000,
        &ctx.dummy_proof(),
        &past_deadline,
    );
 main
}




#[test]
fn test_deposit_zero_amount_fails() {
 error_handling
    let (env, client, admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let project = client.register_project(
        &creator,
        &tokens,
        &1000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    let result = client.try_deposit(&project.id, &creator, &token, &0i128);

    assert_contract_err(result, Error::InvalidAmount);

    let ctx = TestContext::new();
    let (project, token, _) = ctx.setup_project(1000);
    ctx.client
        .deposit(&project.id, &ctx.manager, &token.address, &0i128);
 main
}




#[test]
fn test_deposit_after_deadline_fails() {
 error_handling
    let (env, client, admin) = setup_with_init();
    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone()]);

    let pm = Address::generate(&env);
    client.grant_role(&admin, &pm, &Role::ProjectManager);

    let project = client.register_project(
        &pm,
        &tokens,
        &1000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

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

    let result = client.try_deposit(&project.id, &admin, &token, &100i128);

    assert_contract_err(result, Error::ProjectExpired);
}



// ─── 4. Emergency Pause Tests ────────────────────────────

#[test]
fn test_admin_can_pause_and_unpause() {
    let (_env, client, admin) = setup_with_init();
    
    assert!(!client.is_paused());
    
    client.pause(&admin);
    assert!(client.is_paused());
    
    client.unpause(&admin);
    assert!(!client.is_paused());
}

    let ctx = TestContext::new();
    let (project, token, _) = ctx.setup_project(1000);

    // Fast-forward time
    ctx.jump_time(project.deadline + 1);

    ctx.client
        .deposit(&project.id, &ctx.admin, &token.address, &100i128);
}

#[test]
fn test_admin_can_pause_and_unpause() {
    let ctx = TestContext::new();
    assert!(!ctx.client.is_paused());

    ctx.client.pause(&ctx.admin);
    assert!(ctx.client.is_paused());
 main

    ctx.client.unpause(&ctx.admin);
    assert!(!ctx.client.is_paused());
}

#[test]
fn test_project_exists_and_maybe_load_helpers() {
    let ctx = TestContext::new();
    let contract_id = ctx.client.address.clone();

    // nothing registered yet
    ctx.env.as_contract(&contract_id, || {
        assert!(!crate::storage::project_exists(&ctx.env, 0));
        assert_eq!(crate::storage::maybe_load_project(&ctx.env, 0), None);
    });

    // register one project
    let (project, _, _) = ctx.setup_project(1000);

    ctx.env.as_contract(&contract_id, || {
        assert!(crate::storage::project_exists(&ctx.env, project.id));
        let cfg = crate::storage::maybe_load_project_config(&ctx.env, project.id).unwrap();
        assert_eq!(cfg.id, project.id);

        let st = crate::storage::maybe_load_project_state(&ctx.env, project.id).unwrap();
        assert_eq!(st.donation_count, 0);

        let loaded = crate::storage::maybe_load_project(&ctx.env, project.id).unwrap();
        assert_eq!(loaded.creator, project.creator);
    });
}

#[test]
 error_handling
#[should_panic]
fn test_load_project_pair_panics_for_missing() {
    let (env, _client, _super_admin) = setup_with_init();
    // id 42 not present -> should panic
    crate::storage::load_project_pair(&env, 42);
}

#[test]
fn test_registration_fails_when_paused() {
    let (env, client, admin) = setup_with_init();
    client.pause(&admin);

    let tokens = Vec::from_array(&env, [Address::generate(&env)]);

    let result = client.try_register_project(
        &admin,
        &tokens,
        &1000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    assert_contract_err(result, Error::ProtocolPaused);

#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_non_admin_cannot_pause() {
    let ctx = TestContext::new();
    let rando = ctx.generate_address();
    ctx.client.pause(&rando);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #19)")]
fn test_registration_fails_when_paused() {
    let ctx = TestContext::new();
    ctx.client.pause(&ctx.admin);

    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);
    ctx.register_project(&tokens, 1000);
 main
}




#[test]
fn test_deposit_fails_when_paused() {
 error_handling
    let (env, client, admin) = setup_with_init();

    let token = Address::generate(&env);
    let tokens = Vec::from_array(&env, [token.clone()]);

    let pm = Address::generate(&env);
    client.grant_role(&admin, &pm, &Role::ProjectManager);

    let project = client.register_project(
        &pm,
        &tokens,
        &1000i128,
        &dummy_proof(&env),
        &future_deadline(&env),
    );

    client.pause(&admin);

    let result = client.try_deposit(&project.id, &pm, &token, &100i128);

    assert_contract_err(result, Error::ProtocolPaused);

    let ctx = TestContext::new();
    let (project, token, _) = ctx.setup_project(1000);

    ctx.client.pause(&ctx.admin);
    ctx.client
        .deposit(&project.id, &ctx.manager, &token.address, &100i128);
 main
}



#[test]
fn test_queries_work_when_paused() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client.pause(&ctx.admin);

    // Query should still work
    let loaded = ctx.client.get_project(&project.id);
    assert_eq!(loaded.id, project.id);
}
