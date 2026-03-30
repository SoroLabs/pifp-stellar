extern crate std;

use crate::test_utils::TestContext;

#[test]
fn test_admin_can_pause_and_unpause_project() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    assert!(!ctx.client.get_project(&project.id).paused);

    ctx.client.pause_project(&ctx.admin, &project.id);
    assert!(ctx.client.get_project(&project.id).paused);

    ctx.client.unpause_project(&ctx.admin, &project.id);
    assert!(!ctx.client.get_project(&project.id).paused);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_non_admin_cannot_pause_project() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    let stranger = ctx.generate_address();

    ctx.client.pause_project(&stranger, &project.id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #33)")]
fn test_deposit_fails_when_project_paused() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);
    let donator = ctx.generate_address();

    sac.mint(&donator, &500);
    ctx.client.pause_project(&ctx.admin, &project.id);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #33)")]
fn test_verify_and_release_fails_when_project_paused() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(500);
    let donator = ctx.generate_address();

    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    ctx.client.pause_project(&ctx.admin, &project.id);
    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());
}

#[test]
fn test_project_queries_still_work_when_project_paused() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client.pause_project(&ctx.admin, &project.id);

    let loaded = ctx.client.get_project(&project.id);
    assert_eq!(loaded.id, project.id);
    assert!(loaded.paused);
}
