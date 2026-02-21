
#![cfg(test)]

use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env, vec,
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

// ─── 2. grant_role ───────────────────────────────────────

#[test]
fn test_super_admin_can_grant_admin() {
    let (env, client, super_admin) = setup_with_init();
    let admin = Address::generate(&env);
    client.grant_role(&super_admin, &admin, &Role::Admin);
    assert!(client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_super_admin_can_grant_oracle() {
    let (env, client, super_admin) = setup_with_init();
    let oracle = Address::generate(&env);
    client.grant_role(&super_admin, &oracle, &Role::Oracle);
    assert!(client.has_role(&oracle, &Role::Oracle));
}

#[test]
fn test_super_admin_can_grant_project_manager() {
    let (env, client, super_admin) = setup_with_init();
    let pm = Address::generate(&env);
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    assert!(client.has_role(&pm, &Role::ProjectManager));
}

#[test]
fn test_super_admin_can_grant_auditor() {
    let (env, client, super_admin) = setup_with_init();
    let auditor = Address::generate(&env);
    client.grant_role(&super_admin, &auditor, &Role::Auditor);
    assert!(client.has_role(&auditor, &Role::Auditor));
}

#[test]
fn test_admin_can_grant_project_manager() {
    let (env, client, super_admin) = setup_with_init();
    let admin = Address::generate(&env);
    let pm    = Address::generate(&env);
    client.grant_role(&super_admin, &admin, &Role::Admin);
    client.grant_role(&admin, &pm, &Role::ProjectManager);
    assert!(client.has_role(&pm, &Role::ProjectManager));
}

#[test]
fn test_admin_can_grant_oracle() {
    let (env, client, super_admin) = setup_with_init();
    let admin  = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.grant_role(&super_admin, &admin, &Role::Admin);
    client.grant_role(&admin, &oracle, &Role::Oracle);
    assert!(client.has_role(&oracle, &Role::Oracle));
}

#[test]
#[should_panic]
fn test_admin_cannot_grant_super_admin() {
    let (env, client, super_admin) = setup_with_init();
    let admin    = Address::generate(&env);
    let impostor = Address::generate(&env);
    client.grant_role(&super_admin, &admin, &Role::Admin);
    client.grant_role(&admin, &impostor, &Role::SuperAdmin);
}

#[test]
#[should_panic]
fn test_no_role_cannot_grant() {
    let (env, client, _) = setup_with_init();
    let nobody = Address::generate(&env);
    let target = Address::generate(&env);
    client.grant_role(&nobody, &target, &Role::Admin);
}

#[test]
#[should_panic]
fn test_project_manager_cannot_grant() {
    let (env, client, super_admin) = setup_with_init();
    let pm     = Address::generate(&env);
    let target = Address::generate(&env);
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    client.grant_role(&pm, &target, &Role::Auditor);
}

// ─── 3. revoke_role ──────────────────────────────────────

#[test]
fn test_super_admin_can_revoke_admin() {
    let (env, client, super_admin) = setup_with_init();
    let admin = Address::generate(&env);
    client.grant_role(&super_admin, &admin, &Role::Admin);
    assert!(client.has_role(&admin, &Role::Admin));
    client.revoke_role(&super_admin, &admin);
    assert!(!client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_admin_can_revoke_project_manager() {
    let (env, client, super_admin) = setup_with_init();
    let admin = Address::generate(&env);
    let pm    = Address::generate(&env);
    client.grant_role(&super_admin, &admin, &Role::Admin);
    client.grant_role(&admin, &pm, &Role::ProjectManager);
    client.revoke_role(&admin, &pm);
    assert!(!client.has_role(&pm, &Role::ProjectManager));
}

#[test]
#[should_panic]
fn test_cannot_revoke_super_admin_via_revoke_role() {
    let (_env, client, super_admin) = setup_with_init();
    client.revoke_role(&super_admin, &super_admin);
}

#[test]
#[should_panic]
fn test_project_manager_cannot_revoke() {
    let (env, client, super_admin) = setup_with_init();
    let pm     = Address::generate(&env);
    let target = Address::generate(&env);
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    client.grant_role(&super_admin, &target, &Role::Auditor);
    client.revoke_role(&pm, &target);
}

#[test]
fn test_revoke_no_role_is_noop() {
    let (env, client, super_admin) = setup_with_init();
    let nobody = Address::generate(&env);
    client.revoke_role(&super_admin, &nobody);
    assert_eq!(client.role_of(&nobody), None);
}

// ─── 4. transfer_super_admin ─────────────────────────────

#[test]
fn test_transfer_super_admin() {
    let (env, client, old_super) = setup_with_init();
    let new_super = Address::generate(&env);
    client.transfer_super_admin(&old_super, &new_super);
    assert!(client.has_role(&new_super, &Role::SuperAdmin));
    assert!(!client.has_role(&old_super, &Role::SuperAdmin));
}

// ─── 5. register_project: RBAC gates ────────────────────

#[test]
fn test_project_manager_can_register() {
    let (env, client, super_admin) = setup_with_init();
    let pm       = Address::generate(&env);
    let tokens   = vec![&env, Address::generate(&env)];
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    let project = client.register_project(&pm, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
    assert_eq!(project.creator, pm);
}

#[test]
#[should_panic]
fn test_no_role_cannot_register_project() {
    let (env, client, _) = setup_with_init();
    let nobody = Address::generate(&env);
    let tokens = vec![&env, Address::generate(&env)];
    client.register_project(&nobody, &tokens, &1000i128, &dummy_proof(&env), &future_deadline(&env));
}

// ─── 6. set_oracle + verify_and_release ─────────────────

#[test]
fn test_oracle_can_verify() {
    let (env, client, super_admin) = setup_with_init();
    let oracle = Address::generate(&env);
    let creator = Address::generate(&env);
    let tokens = vec![&env, Address::generate(&env)];
    let proof = dummy_proof(&env);
    
    client.set_oracle(&super_admin, &oracle);
    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    
    let project = client.register_project(&creator, &tokens, &100i128, &proof, &future_deadline(&env));
    client.verify_and_release(&oracle, &project.id, &proof);
    
    let completed = client.get_project(&project.id);
    assert_eq!(completed.status, crate::ProjectStatus::Completed);
}

#[test]
#[should_panic]
fn test_non_oracle_cannot_verify() {
    let (env, client, super_admin) = setup_with_init();
    let pm      = Address::generate(&env);
    let impersonator = Address::generate(&env);
    let tokens = vec![&env, Address::generate(&env)];
    let proof = dummy_proof(&env);
    
    client.grant_role(&super_admin, &pm, &Role::ProjectManager);
    let project = client.register_project(&pm, &tokens, &100i128, &proof, &future_deadline(&env));
    client.verify_and_release(&impersonator, &project.id, &proof);
}