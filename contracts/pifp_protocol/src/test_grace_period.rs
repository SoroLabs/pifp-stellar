extern crate std;

use crate::{test_utils::TestContext, ProjectStatus};

// ─────────────────────────────────────────────────────────
// Happy path: verify_proof → wait 24h → claim_funds
// ─────────────────────────────────────────────────────────

#[test]
fn test_verify_proof_sets_verified_status_and_timestamp() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());

    let updated = ctx.client.get_project(&project.id);
    assert_eq!(updated.status, ProjectStatus::Verified);
    assert!(updated.last_proof_time > 0);
}

#[test]
fn test_claim_funds_after_grace_period_succeeds() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(5000);

    let donator = ctx.generate_address();
    sac.mint(&donator, &1000);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &1000);

    // Verify proof
    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());
    let verified = ctx.client.get_project(&project.id);
    assert_eq!(verified.status, ProjectStatus::Verified);

    // Advance time past 24h grace period
    ctx.jump_time(86_400);

    // Claim funds
    ctx.client.claim_funds(&project.id);

    let completed = ctx.client.get_project(&project.id);
    assert_eq!(completed.status, ProjectStatus::Completed);

    // Creator (manager) received the funds
    assert_eq!(token.balance(&ctx.manager), 1000);
    assert_eq!(token.balance(&ctx.client.address), 0);
}

// ─────────────────────────────────────────────────────────
// GracePeriodActive (#34): claim too early
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_claim_funds_before_grace_period_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // Try to claim immediately — should fail
    ctx.client.claim_funds(&project.id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #34)")]
fn test_claim_funds_one_second_before_grace_period_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // Advance time to 1 second before grace period ends
    ctx.jump_time(86_400 - 1);

    ctx.client.claim_funds(&project.id);
}

// ─────────────────────────────────────────────────────────
// InvalidTransition (#22): claim on non-Verified project
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #22)")]
fn test_claim_funds_on_funding_project_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client.claim_funds(&project.id);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #22)")]
fn test_claim_funds_on_completed_project_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());
    ctx.jump_time(86_400);
    ctx.client.claim_funds(&project.id);

    // Try to claim again
    ctx.client.claim_funds(&project.id);
}

// ─────────────────────────────────────────────────────────
// MilestoneAlreadyReleased (#3): double verify
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_verify_proof_twice_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // Second verify should fail
    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());
}

// ─────────────────────────────────────────────────────────
// Verified project cannot be expired
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #22)")]
fn test_expire_verified_project_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());

    ctx.jump_time(project.deadline + 1);
    ctx.client.expire_project(&project.id);
}

// ─────────────────────────────────────────────────────────
// claim_funds is permissionless
// ─────────────────────────────────────────────────────────

#[test]
fn test_claim_funds_permissionless() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(5000);

    let donator = ctx.generate_address();
    sac.mint(&donator, &500);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500);

    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());
    ctx.jump_time(86_400);

    // Anyone can call claim_funds — no auth required
    ctx.client.claim_funds(&project.id);

    let completed = ctx.client.get_project(&project.id);
    assert_eq!(completed.status, ProjectStatus::Completed);
    assert_eq!(token.balance(&ctx.manager), 500);
}
