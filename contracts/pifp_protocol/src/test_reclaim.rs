extern crate std;

use crate::{test_utils::TestContext, ProjectStatus, REFUND_WINDOW};

// ── reclaim_expired_funds: happy paths ──────────────────────────────

#[test]
fn test_reclaim_after_expiry_window() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    // Donor deposits
    let donator = ctx.generate_address();
    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    // Expire the project
    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);

    let expired = ctx.client.get_project(&project.id);
    assert_eq!(expired.status, ProjectStatus::Expired);
    assert!(expired.refund_expiry > 0);

    // Jump past the refund window
    ctx.jump_time(REFUND_WINDOW + 1);

    // Creator reclaims unclaimed funds
    ctx.client
        .reclaim_expired_funds(&ctx.manager, &project.id);

    // Funds went to creator
    assert_eq!(token.balance(&ctx.manager), 500);
    assert_eq!(token.balance(&ctx.client.address), 0);
    assert_eq!(ctx.client.get_balance(&project.id, &token.address), 0);
}

#[test]
fn test_reclaim_after_cancellation_window() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(500);

    let donator = ctx.generate_address();
    sac.mint(&donator, &600);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &600);

    assert_eq!(
        ctx.client.get_project(&project.id).status,
        ProjectStatus::Active
    );

    // Cancel the project
    ctx.client.cancel_project(&ctx.manager, &project.id);

    let cancelled = ctx.client.get_project(&project.id);
    assert_eq!(cancelled.status, ProjectStatus::Cancelled);
    assert!(cancelled.refund_expiry > 0);

    // Jump past refund window
    ctx.jump_time(REFUND_WINDOW + 1);

    ctx.client
        .reclaim_expired_funds(&ctx.manager, &project.id);

    assert_eq!(token.balance(&ctx.manager), 600);
    assert_eq!(ctx.client.get_balance(&project.id, &token.address), 0);
}

#[test]
fn test_partial_refund_then_reclaim_remainder() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    let donator_a = ctx.generate_address();
    let donator_b = ctx.generate_address();
    sac.mint(&donator_a, &300);
    sac.mint(&donator_b, &200);

    ctx.client
        .deposit(&project.id, &donator_a, &token.address, &300);
    ctx.client
        .deposit(&project.id, &donator_b, &token.address, &200);

    // Expire
    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);

    // donator_a claims refund within the window
    ctx.client.refund(&donator_a, &project.id, &token.address);
    assert_eq!(token.balance(&donator_a), 300);

    // Jump past refund window — donator_b did not claim
    ctx.jump_time(REFUND_WINDOW + 1);

    // Creator reclaims the remaining 200
    ctx.client
        .reclaim_expired_funds(&ctx.manager, &project.id);
    assert_eq!(token.balance(&ctx.manager), 200);
    assert_eq!(ctx.client.get_balance(&project.id, &token.address), 0);
}

// ── reclaim_expired_funds: failure paths ────────────────────────────

#[test]
#[should_panic(expected = "Error(Contract, #24)")]
fn test_reclaim_before_window_expires_fails() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    let donator = ctx.generate_address();
    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);

    // Try to reclaim during the refund window — should fail
    ctx.client
        .reclaim_expired_funds(&ctx.manager, &project.id);
}

#[test]
#[should_panic(expected = "Error(Contract, #6)")]
fn test_reclaim_by_non_creator_fails() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    let donator = ctx.generate_address();
    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);
    ctx.jump_time(REFUND_WINDOW + 1);

    // Non-creator tries to reclaim — should fail
    let attacker = ctx.generate_address();
    ctx.client
        .reclaim_expired_funds(&attacker, &project.id);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_reclaim_on_funding_project_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // Project is still Funding — should fail
    ctx.client
        .reclaim_expired_funds(&ctx.manager, &project.id);
}

#[test]
#[should_panic(expected = "Error(Contract, #22)")]
fn test_reclaim_on_completed_project_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // Verify and release — project becomes Completed
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    ctx.jump_time(REFUND_WINDOW + 1);

    ctx.client
        .reclaim_expired_funds(&ctx.manager, &project.id);
}

// ── refund window expiry: donor blocked after window ────────────────

#[test]
#[should_panic(expected = "Error(Contract, #25)")]
fn test_donor_refund_blocked_after_window_expires() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    let donator = ctx.generate_address();
    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);

    // Jump past refund window
    ctx.jump_time(REFUND_WINDOW + 1);

    // Donor tries to refund after window — should fail
    ctx.client.refund(&donator, &project.id, &token.address);
}

#[test]
fn test_donor_refund_allowed_within_window() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);

    let donator = ctx.generate_address();
    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);

    // Refund within the window — should succeed
    ctx.client.refund(&donator, &project.id, &token.address);
    assert_eq!(token.balance(&donator), 500);
}

// ── refund_expiry field is set correctly ─────────────────────────────

#[test]
fn test_refund_expiry_set_on_expire() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.jump_time(86_401);
    ctx.client.expire_project(&project.id);

    let p = ctx.client.get_project(&project.id);
    // refund_expiry = time of expire + REFUND_WINDOW
    assert_eq!(p.refund_expiry, 100_000 + 86_401 + REFUND_WINDOW);
}

#[test]
fn test_refund_expiry_set_on_cancel() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(500);

    let donator = ctx.generate_address();
    sac.mint(&donator, &600);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &600);

    ctx.client.cancel_project(&ctx.manager, &project.id);

    let p = ctx.client.get_project(&project.id);
    // refund_expiry = current timestamp + REFUND_WINDOW
    assert_eq!(p.refund_expiry, 100_000 + REFUND_WINDOW);
}

#[test]
fn test_refund_expiry_zero_on_new_project() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    assert_eq!(project.refund_expiry, 0);
}
