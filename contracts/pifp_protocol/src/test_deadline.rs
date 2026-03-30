extern crate std;

use crate::{test_utils::TestContext, Role};
use soroban_sdk::Vec;

fn register(ctx: &TestContext, deadline: u64) -> crate::types::Project {
    let (token, _) = ctx.create_token();
    let tokens = Vec::from_array(&ctx.env, [token.address.clone()]);
    let empty_oracles: soroban_sdk::Vec<soroban_sdk::Address> = soroban_sdk::Vec::new(&ctx.env);
    ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &1000i128,
        &ctx.dummy_proof(),
        &ctx.dummy_metadata_uri(),
        &deadline,
        &false,
        &empty_oracles,
        &0u32,
    )
}

#[test]
fn test_extend_deadline_success() {
    let ctx = TestContext::new();
    let deadline = ctx.env.ledger().timestamp() + 10000;
    let project = register(&ctx, deadline);
    let new_deadline = deadline + 5000;
    ctx.client.extend_deadline(&ctx.manager, &project.id, &new_deadline);
    assert_eq!(ctx.client.get_project(&project.id).deadline, new_deadline);
}

#[test]
fn test_extend_deadline_by_admin() {
    let ctx = TestContext::new();
    let deadline = ctx.env.ledger().timestamp() + 10000;
    let project = register(&ctx, deadline);
    let new_deadline = deadline + 5000;
    ctx.client.extend_deadline(&ctx.admin, &project.id, &new_deadline);
    assert_eq!(ctx.client.get_project(&project.id).deadline, new_deadline);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_extend_deadline_unauthorized() {
    let ctx = TestContext::new();
    let deadline = ctx.env.ledger().timestamp() + 10000;
    let project = register(&ctx, deadline);
    let stranger = ctx.generate_address();
    ctx.client.extend_deadline(&stranger, &project.id, &(deadline + 5000));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_extend_deadline_backwards() {
    let ctx = TestContext::new();
    let deadline = ctx.env.ledger().timestamp() + 10000;
    let project = register(&ctx, deadline);
    // Same deadline — not strictly later, should fail with InvalidDeadline.
    ctx.client.extend_deadline(&ctx.manager, &project.id, &deadline);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_extend_deadline_expired() {
    let ctx = TestContext::new();
    let deadline = ctx.env.ledger().timestamp() + 10000;
    let project = register(&ctx, deadline);
    ctx.jump_time(10001);
    ctx.client.extend_deadline(&ctx.manager, &project.id, &(deadline + 5000));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #24)")]
fn test_extend_deadline_too_long() {
    let ctx = TestContext::new();
    let deadline = ctx.env.ledger().timestamp() + 10000;
    let project = register(&ctx, deadline);
    // 1 year + 1 second from now
    let too_late = ctx.env.ledger().timestamp() + 31_536_000 + 1;
    ctx.client.extend_deadline(&ctx.manager, &project.id, &too_late);
}
