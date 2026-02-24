extern crate std;

use soroban_sdk::{symbol_short, testutils::Events, vec, IntoVal, TryIntoVal};

use crate::events::{ProjectCreated, ProjectFunded, ProjectVerified};
use crate::test_utils::TestContext;

#[test]
fn test_project_created_event() {
    let ctx = TestContext::new();
    let (project, token, _) = ctx.setup_project(5000);

    let all_events = ctx.env.events().all();
    let last_event = all_events.last().expect("No events found");

    // Topic: (symbol_short!("created"), project_id)
    assert_eq!(last_event.0, ctx.client.address);
    let expected_topics = vec![
        &ctx.env,
        symbol_short!("created").into_val(&ctx.env),
        project.id.into_val(&ctx.env),
    ];
    assert_eq!(last_event.1, expected_topics);

    // Data: ProjectCreated struct
    let event_data: ProjectCreated = last_event.2.try_into_val(&ctx.env).unwrap();
    assert_eq!(
        event_data,
        ProjectCreated {
            project_id: project.id,
            creator: ctx.manager.clone(),
            token: token.address.clone(),
            goal: 5000,
        }
    );
}

#[test]
fn test_project_funded_event() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(10000);

    let donator = ctx.generate_address();
    let amount = 1000i128;
    sac.mint(&donator, &amount);

    ctx.client
        .deposit(&project.id, &donator, &token.address, &amount);

    let all_events = ctx.env.events().all();
    let last_event = all_events.last().expect("No events found");

    // Topic: (symbol_short!("funded"), project_id)
    assert_eq!(last_event.0, ctx.client.address);
    let expected_topics = vec![
        &ctx.env,
        symbol_short!("funded").into_val(&ctx.env),
        project.id.into_val(&ctx.env),
    ];
    assert_eq!(last_event.1, expected_topics);

    // Data: ProjectFunded struct
    let event_data: ProjectFunded = last_event.2.try_into_val(&ctx.env).unwrap();
    assert_eq!(
        event_data,
        ProjectFunded {
            project_id: project.id,
            donator: donator.clone(),
            amount,
        }
    );
}

#[test]
fn test_project_verified_event() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(1000);
    let proof = ctx.dummy_proof();

    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &proof);

    let all_events = ctx.env.events().all();
    let last_event = all_events.last().expect("No events found");

    // Topic: (symbol_short!("verified"), project_id)
    assert_eq!(last_event.0, ctx.client.address);
    let expected_topics = vec![
        &ctx.env,
        symbol_short!("verified").into_val(&ctx.env),
        project.id.into_val(&ctx.env),
    ];
    assert_eq!(last_event.1, expected_topics);

    // Data: ProjectVerified struct
    let event_data: ProjectVerified = last_event.2.try_into_val(&ctx.env).unwrap();
    assert_eq!(
        event_data,
        ProjectVerified {
            project_id: project.id,
            oracle: ctx.oracle.clone(),
            proof_hash: proof.clone(),
        }
    );
}

#[test]
fn test_get_project_balances() {
    let ctx = TestContext::new();

    // Create two distinct SAC tokens
    let (token_a, sac_a) = ctx.create_token();
    let (token_b, sac_b) = ctx.create_token();

    // Grant manager and register project with two tokens
    let tokens = vec![&ctx.env, token_a.address.clone(), token_b.address.clone()];
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &10_000,
        &ctx.dummy_proof(),
        &(ctx.env.ledger().timestamp() + 86400),
    );

    let donator = ctx.generate_address();
    let amount_a = 2_500i128;
    let amount_b = 7_000i128;

    sac_a.mint(&donator, &amount_a);
    sac_b.mint(&donator, &amount_b);

    ctx.client
        .deposit(&project.id, &donator, &token_a.address, &amount_a);
    ctx.client
        .deposit(&project.id, &donator, &token_b.address, &amount_b);

    // Query balances
    let balances = ctx.client.get_project_balances(&project.id);

    assert_eq!(balances.project_id, project.id);
    assert_eq!(balances.balances.len(), 2);

    let bal_a = balances.balances.get(0).unwrap();
    let bal_b = balances.balances.get(1).unwrap();

    assert_eq!(bal_a.token, token_a.address);
    assert_eq!(bal_a.balance, amount_a);
    assert_eq!(bal_b.token, token_b.address);
    assert_eq!(bal_b.balance, amount_b);
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

    // Verify and release
    ctx.client
        .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

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
    ctx.client
        .deposit(&project.id, &donator, &token.address, &400i128);

    ctx.jump_time(86_401);
    ctx.client.refund(&donator, &project.id, &token.address);

    let all_events = ctx.env.events().all();
    let last_event = all_events.last().expect("No events found");

    assert_eq!(last_event.0, ctx.client.address);
    let expected_topics = vec![
        &ctx.env,
        symbol_short!("refunded").into_val(&ctx.env),
        project.id.into_val(&ctx.env),
    ];
    assert_eq!(last_event.1, expected_topics);

    let event_data: (soroban_sdk::Address, i128) = last_event.2.try_into_val(&ctx.env).unwrap();
    assert_eq!(event_data.0, donator);
    assert_eq!(event_data.1, 400i128);
}
