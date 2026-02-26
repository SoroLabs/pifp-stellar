extern crate std;

 error_handling
#![cfg(test)]

extern crate std;
extern crate alloc;

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, vec, InvokeError,
};

use crate::{PifpProtocol, PifpProtocolClient, Role, Error};

// ─── Helpers ─────────────────────────────────────────────

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

fn dummy_proof(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xabu8; 32])
}

fn future_deadline(env: &Env) -> u64 {
    env.ledger().timestamp() + 86_400
}

use alloc::string::String;
use core::fmt::Write as _;

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

use crate::{test_utils::TestContext, Role};
use soroban_sdk::vec;
 main

#[test]
fn test_init_sets_super_admin() {
    let ctx = TestContext::new();
    assert!(ctx.client.has_role(&ctx.admin, &Role::SuperAdmin));
}

#[test]
fn test_super_admin_can_grant_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();
    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    assert!(ctx.client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_super_admin_can_grant_oracle() {
    let ctx = TestContext::new();
    let oracle = ctx.generate_address();
    ctx.client.grant_role(&ctx.admin, &oracle, &Role::Oracle);
    assert!(ctx.client.has_role(&oracle, &Role::Oracle));
}

#[test]
fn test_admin_can_grant_project_manager() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();
    let pm = ctx.generate_address();

    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    ctx.client.grant_role(&admin, &pm, &Role::ProjectManager);
    assert!(ctx.client.has_role(&pm, &Role::ProjectManager));
}

#[test]
#[should_panic]
fn test_admin_cannot_grant_super_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();
    let impostor = ctx.generate_address();

    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    ctx.client.grant_role(&admin, &impostor, &Role::SuperAdmin);
}

#[test]
 error_handling
fn test_project_manager_cannot_grant() {
    let (env, client, super_admin) = setup_with_init();
    let pm = Address::generate(&env);
    let target = Address::generate(&env);
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);

    let result = client.try_grant_role(&pm, &target, &Role::Auditor);

    assert_contract_err(result, Error::NotAuthorized);
}

// ─── 3. revoke_role ──────────────────────────────────────

#[test]

 main
fn test_super_admin_can_revoke_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();

 error_handling
#[test]
#[should_panic]
fn test_cannot_revoke_super_admin_via_revoke_role() {
    let (_env, client, super_admin) = setup_with_init();
    client.revoke_role(&super_admin, &super_admin);
}

#[test]
fn test_project_manager_cannot_revoke() {
    let (env, client, super_admin) = setup_with_init();
    let pm = Address::generate(&env);
    let target = Address::generate(&env);
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    client.grant_role(&super_admin, &target, &Role::Auditor);

    let result = client.try_revoke_role(&pm, &target);

    assert_contract_err(result, Error::NotAuthorized);
}

    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    assert!(ctx.client.has_role(&admin, &Role::Admin));
 main

    ctx.client.revoke_role(&ctx.admin, &admin);
    assert!(!ctx.client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_transfer_super_admin() {
    let ctx = TestContext::new();
    let new_super = ctx.generate_address();

    ctx.client.transfer_super_admin(&ctx.admin, &new_super);
    assert!(ctx.client.has_role(&new_super, &Role::SuperAdmin));
    assert!(!ctx.client.has_role(&ctx.admin, &Role::SuperAdmin));
}

#[test]
fn test_project_manager_can_register() {
    let ctx = TestContext::new();
    let tokens = vec![&ctx.env, ctx.generate_address()];

    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &(ctx.env.ledger().timestamp() + 86400),
    );
    assert_eq!(project.creator, ctx.manager);
}

#[test]
fn test_oracle_can_verify() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(100);

    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    let completed = ctx.client.get_project(&project.id);
    assert_eq!(completed.status, crate::ProjectStatus::Completed);
}
