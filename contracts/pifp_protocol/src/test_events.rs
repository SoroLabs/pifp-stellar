extern crate std;

use soroban_sdk::{vec, BytesN, Vec, Address};

use crate::test_utils::TestContext;

#[test]
fn test_project_created_event() {
    let ctx = TestContext::new();
    let (_project, _token, _) = ctx.setup_project(5000);
}

#[test]
fn test_project_funded_event() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(10000);
    let donator = ctx.generate_address();
    sac.mint(&donator, &1000i128);
    ctx.client.deposit(&project.id, &donator, &token.address, &1000i128);
}

#[test]
fn test_project_verified_event() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    let proof = ctx.dummy_proof();
    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &proof);
}

#[test]
fn test_get_project_balances() {
    let ctx = TestContext::new();
    let (token_a, sac_a) = ctx.create_token();
    let (token_b, sac_b) = ctx.create_token();
    let env = &ctx.env;
    let tokens = vec![env, token_a.address.clone(), token_b.address.clone()];
    
    let proof_hash = ctx.dummy_proof();
    let metadata_uri = ctx.dummy_metadata_uri();
    let deadline = env.ledger().timestamp() + 86400;
    
    let mut milestones = Vec::new(env);
    milestones.push_back(crate::types::Milestone {
        label: BytesN::from_array(env, &[0u8; 32]),
        amount_bps: 10000,
        proof_hash: proof_hash.clone(),
    });

    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &10_000i128,
        &proof_hash,
        &metadata_uri,
        &deadline,
        &false,
        &milestones,
        &0u32,
        &Vec::new(env),
        &0u32,
    );

    let donator = ctx.generate_address();
    sac_a.mint(&donator, &2_500i128);
    sac_b.mint(&donator, &7_000i128);
    ctx.client.deposit(&project.id, &donator, &token_a.address, &2_500i128);
    ctx.client.deposit(&project.id, &donator, &token_b.address, &7_000i128);

    let balances = ctx.client.get_project_balances(&project.id);
    assert_eq!(balances.project_id, project.id);
    assert_eq!(balances.balances.len(), 2);
    assert_eq!(balances.balances.get(0).unwrap().balance, 2_500i128);
    assert_eq!(balances.balances.get(1).unwrap().balance, 7_000i128);
}

#[test]
fn test_funds_released_to_creator() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(5000);
    let donator = ctx.generate_address();
    let deposit_amount = 1000i128;
    sac.mint(&donator, &deposit_amount);

    ctx.client
        .deposit(&project.id, &donator, &token.address, &deposit_amount);

    // Verify
    ctx.client
        .verify_proof(&ctx.oracle, &project.id, &ctx.dummy_proof());
        
    // Claim after grace period
    ctx.jump_time(86_400); // grace period
    ctx.client.claim_funds(&project.id);

    // Check creator (manager) received the funds
    assert_eq!(token.balance(&ctx.manager), deposit_amount);

    // Check contract no longer has the funds
    assert_eq!(token.balance(&ctx.client.address), 0);
}

#[test]
fn test_refunded_event() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(1000);
    let donator = ctx.generate_address();
    sac.mint(&donator, &400i128);
    ctx.client.deposit(&project.id, &donator, &token.address, &400i128);
    ctx.jump_time(86_401);
    ctx.client.refund(&donator, &project.id, &token.address);
}
