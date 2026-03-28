extern crate std;

use crate::test_utils::TestContext;
use soroban_sdk::{BytesN, Vec};

// ─────────────────────────────────────────────────────────
// ProjectNotFound (#1)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_get_project_not_found() {
    let ctx = TestContext::new();
    ctx.client.get_project(&999);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_deposit_on_nonexistent_project() {
    let ctx = TestContext::new();
    let token = ctx.generate_address();
    ctx.client.deposit(&42, &ctx.manager, &token, &100i128);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #1)")]
fn test_get_project_balances_not_found() {
    let ctx = TestContext::new();
    ctx.client.get_project_balances(&999);
}

// ─────────────────────────────────────────────────────────
// MilestoneAlreadyReleased (#3)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #3)")]
fn test_verify_already_completed_project() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // First verification succeeds.
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // Second verification must fail with MilestoneAlreadyReleased.
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());
}

// ─────────────────────────────────────────────────────────
// InvalidGoal (#7)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_register_negative_goal_fails() {
    let ctx = TestContext::new();
    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);
    ctx.register_project(&tokens, -100);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #7)")]
fn test_register_goal_exceeds_upper_bound_fails() {
    let ctx = TestContext::new();
    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);
    // 10^30 + 1 — exceeds upper bound
    let huge_goal: i128 = 1_000_000_000_000_000_000_000_000_000_001;
    ctx.register_project(&tokens, huge_goal);
}

// ─────────────────────────────────────────────────────────
// TooManyTokens (#10)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #10)")]
fn test_register_too_many_tokens_fails() {
    let ctx = TestContext::new();
    let mut tokens = Vec::new(&ctx.env);
    for _ in 0..11 {
        tokens.push_back(ctx.generate_address());
    }
    ctx.register_project(&tokens, 1000);
}

// ─────────────────────────────────────────────────────────
// InvalidDeadline (#13)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_register_deadline_too_far_in_future_fails() {
    let ctx = TestContext::new();
    let tokens = Vec::from_array(&ctx.env, [ctx.generate_address()]);
    // More than 5 years in the future
    let too_far_deadline = ctx.env.ledger().timestamp() + 200_000_000;
    ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000,
        &ctx.dummy_proof(),
        &too_far_deadline,
    );
}

// ─────────────────────────────────────────────────────────
// VerificationFailed (#16)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #16)")]
fn test_verify_wrong_proof_hash_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    let wrong_proof = BytesN::from_array(&ctx.env, &[0xffu8; 32]);
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &wrong_proof);
}

// ─────────────────────────────────────────────────────────
// EmptyAcceptedTokens (#17)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #17)")]
fn test_register_empty_tokens_fails() {
    let ctx = TestContext::new();
    let tokens: Vec<soroban_sdk::Address> = Vec::new(&ctx.env);
    ctx.register_project(&tokens, 1000);
}

// ─────────────────────────────────────────────────────────
// ProtocolPaused (#19)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #19)")]
fn test_verify_when_paused_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    ctx.client.pause(&ctx.admin);
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());
}

// ─────────────────────────────────────────────────────────
// ProjectNotExpired (#21)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #21)")]
fn test_expire_project_before_deadline_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    // No time jump — deadline has not passed.
    ctx.client.expire_project(&project.id);
}

// ─────────────────────────────────────────────────────────
// InvalidTransition (#22)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #22)")]
fn test_expire_completed_project_fails_with_invalid_transition() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);

    // Complete the project.
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // Attempt to expire it — should fail with InvalidTransition.
    ctx.jump_time(project.deadline + 1);
    ctx.client.expire_project(&project.id);
}

// ─────────────────────────────────────────────────────────
// TokenNotAccepted (#23)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #23)")]
fn test_deposit_unaccepted_token_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    let rogue_token = ctx.generate_address();

    ctx.client
        .deposit(&project.id, &ctx.manager, &rogue_token, &100i128);
}

// ─────────────────────────────────────────────────────────
// NotAuthorized (#6)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_admin_cannot_cancel_project() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(500);
    let donator = ctx.generate_address();
    let other_admin = ctx.generate_address();

    ctx.client
        .grant_role(&ctx.admin, &other_admin, &crate::Role::Admin);
    sac.mint(&donator, &600i128);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &600i128);

    ctx.client.cancel_project(&other_admin, &project.id);
}

// ─────────────────────────────────────────────────────────
// InvalidTransition (#22)
// ─────────────────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #22)")]
fn test_cancel_non_active_project_fails() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    ctx.client.cancel_project(&ctx.manager, &project.id);
}
