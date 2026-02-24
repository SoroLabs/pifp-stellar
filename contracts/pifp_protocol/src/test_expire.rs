extern crate std;

use crate::{test_utils::TestContext, ProjectStatus};

#[test]
fn test_expire_project_success() {
    let ctx = TestContext::new();
    let (project, _token, _) = ctx.setup_project(1000);

    assert_eq!(project.status, ProjectStatus::Funding);

    // Jump forward in time
    ctx.jump_time(project.deadline + 1);

    ctx.client.expire_project(&project.id);

    let expired_project = ctx.client.get_project(&project.id);
    assert_eq!(expired_project.status, ProjectStatus::Expired);
}

#[test]
#[should_panic]
fn test_expire_before_deadline_panics() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // Attempt to expire before deadline
    ctx.client.expire_project(&project.id);
}

#[test]
#[should_panic]
fn test_expire_wrong_status_panics() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // Expire properly first
    ctx.jump_time(project.deadline + 1);
    ctx.client.expire_project(&project.id);

    // Attempt to expire again (Expired status is wrong status for expire_project)
    ctx.client.expire_project(&project.id);
}

#[test]
#[should_panic]
fn test_expire_completed_project_panics() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // Move to Completed
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // Attempt to expire
    ctx.jump_time(project.deadline + 1);
    ctx.client.expire_project(&project.id);
}

#[test]
fn test_expire_active_project_success() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    // Deposit to make it Active
    sac.mint(&ctx.admin, &1000);
    ctx.client
        .deposit(&project.id, &ctx.admin, &token.address, &1000);

    let active_project = ctx.client.get_project(&project.id);
    assert_eq!(active_project.status, ProjectStatus::Active);

    // Jump forward in time
    ctx.jump_time(project.deadline + 1);

    ctx.client.expire_project(&project.id);

    let expired_project = ctx.client.get_project(&project.id);
    assert_eq!(expired_project.status, ProjectStatus::Expired);
}
