extern crate std;

use crate::{test_utils::TestContext, Role};
use soroban_sdk::vec;

#[test]
fn test_init_sets_super_admin() {
    let ctx = TestContext::new();
    assert!(ctx.client.has_role(&ctx.admin, &Role::SuperAdmin));
}

#[test]
fn test_super_admin_can_grant_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();
    ctx.mock_auth(&ctx.admin, "grant_role", (&ctx.admin, &admin, Role::Admin));
    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    assert!(ctx.client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_super_admin_can_grant_oracle() {
    let ctx = TestContext::new();
    let oracle = ctx.generate_address();
    ctx.mock_auth(&ctx.admin, "grant_role", (&ctx.admin, &oracle, Role::Oracle));
    ctx.client.grant_role(&ctx.admin, &oracle, &Role::Oracle);
    assert!(ctx.client.has_role(&oracle, &Role::Oracle));
}

#[test]
fn test_admin_can_grant_project_manager() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();
    let pm = ctx.generate_address();

    ctx.mock_auth(&ctx.admin, "grant_role", (&ctx.admin, &admin, Role::Admin));
    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    ctx.mock_auth(&admin, "grant_role", (&admin, &pm, Role::ProjectManager));
    ctx.client.grant_role(&admin, &pm, &Role::ProjectManager);
    assert!(ctx.client.has_role(&pm, &Role::ProjectManager));
}

#[test]
#[should_panic]
fn test_admin_cannot_grant_super_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();
    let impostor = ctx.generate_address();

    ctx.mock_auth(&ctx.admin, "grant_role", (&ctx.admin, &admin, Role::Admin));
    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    ctx.mock_auth(&admin, "grant_role", (&admin, &impostor, Role::SuperAdmin));
    ctx.client.grant_role(&admin, &impostor, &Role::SuperAdmin);
}

#[test]
fn test_super_admin_can_revoke_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();

    ctx.mock_auth(&ctx.admin, "grant_role", (&ctx.admin, &admin, Role::Admin));
    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    assert!(ctx.client.has_role(&admin, &Role::Admin));

    ctx.mock_auth(&ctx.admin, "revoke_role", (&ctx.admin, &admin));
    ctx.client.revoke_role(&ctx.admin, &admin);
    assert!(!ctx.client.has_role(&admin, &Role::Admin));
}

#[test]
fn test_transfer_super_admin() {
    let ctx = TestContext::new();
    let new_super = ctx.generate_address();

    ctx.mock_auth(&ctx.admin, "transfer_super_admin", (&ctx.admin, &new_super));
    ctx.client.transfer_super_admin(&ctx.admin, &new_super);
    assert!(ctx.client.has_role(&new_super, &Role::SuperAdmin));
    assert!(!ctx.client.has_role(&ctx.admin, &Role::SuperAdmin));
}

#[test]
fn test_project_manager_can_register() {
    let ctx = TestContext::new();
    let tokens = soroban_sdk::Vec::from_array(&ctx.env, [ctx.generate_address()]);
    let metadata_uri = ctx.dummy_metadata_uri();
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &metadata_uri,
        &(ctx.env.ledger().timestamp() + 86400),
        &false,
        &{
            let mut ms = soroban_sdk::Vec::new(&ctx.env);
            ms.push_back(crate::types::Milestone {
                label: soroban_sdk::BytesN::from_array(&ctx.env, &[0u8; 32]),
                amount_bps: 10000,
                proof_hash: ctx.dummy_proof().clone(),
            });
            ms
        },
        &0u32,
        &soroban_sdk::Vec::new(&ctx.env),
        &0u32,
    );
    assert_eq!(project.creator, ctx.manager);
}

#[test]
fn test_oracle_can_verify() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(100);

    ctx.mock_auth(&ctx.oracle, "verify_proof", (&ctx.oracle, project.id, ctx.dummy_proof()));
    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());

    let completed = ctx.client.get_project(&project.id);
    assert_eq!(completed.status, crate::ProjectStatus::Verified);
}
