extern crate std;

use crate::{test_utils::TestContext, ProjectStatus, Role};
use soroban_sdk::Vec;

/// Helper: register a project with an explicit M-of-N oracle set.
fn register_with_oracles(
    ctx: &TestContext,
    oracles: &Vec<soroban_sdk::Address>,
    threshold: u32,
) -> crate::types::Project {
    let (token, _) = ctx.create_token();
    let tokens = soroban_sdk::Vec::from_array(&ctx.env, [token.address.clone()]);
    ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &ctx.dummy_metadata_uri(),
        &(ctx.env.ledger().timestamp() + 86400),
        &false,
        oracles,
        &threshold,
    )
}

// ── Happy path: 2-of-3 ───────────────────────────────────────────────────────

#[test]
fn test_two_of_three_releases_on_second_vote() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let o2 = ctx.generate_address();
    let o3 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone(), o2.clone(), o3.clone()]);

    let project = register_with_oracles(&ctx, &oracles, 2);

    // First vote — not yet at threshold.
    ctx.client.verify_and_release(&o1, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Funding);

    // Second vote — threshold met, funds released.
    ctx.client.verify_and_release(&o2, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Completed);
}

// ── Duplicate vote prevention ─────────────────────────────────────────────────

#[test]
fn test_duplicate_vote_does_not_double_count() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let o2 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone(), o2.clone()]);

    // 2-of-2 threshold.
    let project = register_with_oracles(&ctx, &oracles, 2);

    // o1 votes twice — second vote must be a no-op on voter_count.
    ctx.client.verify_and_release(&o1, &project.id, &ctx.dummy_proof());
    ctx.client.verify_and_release(&o1, &project.id, &ctx.dummy_proof());

    // Still Funding — only 1 unique vote counted.
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Funding);

    // o2 votes — now 2 unique votes, threshold met.
    ctx.client.verify_and_release(&o2, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Completed);
}

// ── Unauthorized oracle rejected ──────────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #28)")]
fn test_unauthorized_oracle_rejected() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone()]);
    let project = register_with_oracles(&ctx, &oracles, 1);

    let rogue = ctx.generate_address();
    ctx.client.verify_and_release(&rogue, &project.id, &ctx.dummy_proof());
}

// ── ThresholdAlreadyMet after completion ──────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #29)")]
fn test_vote_after_threshold_met_panics() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone()]);
    let project = register_with_oracles(&ctx, &oracles, 1);

    // First vote completes the project.
    ctx.client.verify_and_release(&o1, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Completed);

    // Second vote must fail with ThresholdAlreadyMet.
    ctx.client.verify_and_release(&o1, &project.id, &ctx.dummy_proof());
}

// ── add_oracle / remove_oracle ────────────────────────────────────────────────

#[test]
fn test_add_oracle_and_vote() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone()]);
    let project = register_with_oracles(&ctx, &oracles, 1);

    // Add a second oracle post-registration.
    let o2 = ctx.generate_address();
    ctx.client.add_oracle(&ctx.admin, &project.id, &o2);

    // Update threshold to 2-of-2 via a new registration isn't possible,
    // but we can verify o2 can now vote (threshold is still 1 from registration).
    ctx.client.verify_and_release(&o2, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Completed);
}

#[test]
fn test_remove_oracle_resets_agreement() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let o2 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone(), o2.clone()]);
    let project = register_with_oracles(&ctx, &oracles, 2);

    // o1 votes.
    ctx.client.verify_and_release(&o1, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Funding);

    // Admin removes o1 — agreement is reset.
    ctx.client.remove_oracle(&ctx.admin, &project.id, &o1);

    // o2 is now at index 0; o1's old vote is gone.
    // o2 votes — but threshold is still 2 and only 1 oracle remains, so it won't release.
    // (This tests that the reset happened — o2's vote alone won't meet threshold=2.)
    ctx.client.verify_and_release(&o2, &project.id, &ctx.dummy_proof());
    assert_eq!(ctx.client.get_project(&project.id).status, ProjectStatus::Funding);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #28)")]
fn test_remove_nonexistent_oracle_fails() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone()]);
    let project = register_with_oracles(&ctx, &oracles, 1);

    let ghost = ctx.generate_address();
    ctx.client.remove_oracle(&ctx.admin, &project.id, &ghost);
}

// ── InvalidOracleConfig validation ───────────────────────────────────────────

#[test]
#[should_panic(expected = "HostError: Error(Contract, #30)")]
fn test_threshold_exceeds_oracle_count_fails() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone()]);
    // threshold=2 but only 1 oracle — invalid.
    register_with_oracles(&ctx, &oracles, 2);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #30)")]
fn test_zero_threshold_with_oracles_fails() {
    let ctx = TestContext::new();
    let o1 = ctx.generate_address();
    let oracles = soroban_sdk::Vec::from_array(&ctx.env, [o1.clone()]);
    register_with_oracles(&ctx, &oracles, 0);
}
