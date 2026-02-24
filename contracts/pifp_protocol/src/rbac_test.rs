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
fn test_super_admin_can_revoke_admin() {
    let ctx = TestContext::new();
    let admin = ctx.generate_address();

    ctx.client.grant_role(&ctx.admin, &admin, &Role::Admin);
    assert!(ctx.client.has_role(&admin, &Role::Admin));

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
