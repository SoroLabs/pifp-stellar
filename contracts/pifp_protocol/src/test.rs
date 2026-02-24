extern crate std;

use crate::{test_utils::TestContext, ProjectStatus, Role};
use soroban_sdk::Vec;

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
#[should_panic(expected = "HostError: Error(Contract, #12)")]
fn test_register_duplicate_tokens_fails() {
    let ctx = TestContext::new();
    let token = ctx.generate_address();
    let tokens = Vec::from_array(&ctx.env, [token.clone(), token.clone()]);

    ctx.register_project(&tokens, 1000);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_register_zero_goal_fails() {
    let ctx = TestContext::new();
    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);
    ctx.register_project(&tokens, 0);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_register_past_deadline_fails() {
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
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #11)")]
fn test_deposit_zero_amount_fails() {
    let ctx = TestContext::new();
    let (project, token, _) = ctx.setup_project(1000);
    ctx.client
        .deposit(&project.id, &ctx.manager, &token.address, &0i128);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_deposit_after_deadline_fails() {
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
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #19)")]
fn test_deposit_fails_when_paused() {
    let ctx = TestContext::new();
    let (project, token, _) = ctx.setup_project(1000);

    ctx.client.pause(&ctx.admin);
    ctx.client
        .deposit(&project.id, &ctx.manager, &token.address, &100i128);
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
