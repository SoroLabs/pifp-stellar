extern crate std;

use crate::test_utils::TestContext;

#[test]
fn test_donation_count_initialized_to_zero() {
    let ctx = TestContext::new();
    let (project, _, _) = ctx.setup_project(10000);
    assert_eq!(project.donation_count, 0);
}

#[test]
fn test_donation_count_increments_for_new_donor() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(10000);
    let donator = ctx.generate_address();

    sac.mint(&donator, &1_000);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500i128);

    let updated = ctx.client.get_project(&project.id);
    assert_eq!(updated.donation_count, 1);
}

#[test]
fn test_donation_count_stays_same_for_repeated_donor() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(10000);
    let donator = ctx.generate_address();

    sac.mint(&donator, &2_000);
    ctx.client
        .deposit(&project.id, &donator, &token.address, &500i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 1);

    // Second deposit from same donor with same token
    ctx.client
        .deposit(&project.id, &donator, &token.address, &300i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 1);
}

#[test]
fn test_donation_count_increments_for_different_donors() {
    let ctx = TestContext::new();
    let (project, token, sac) = ctx.setup_project(10000);
    let donator1 = ctx.generate_address();
    let donator2 = ctx.generate_address();

    sac.mint(&donator1, &1_000);
    sac.mint(&donator2, &1_000);

    ctx.client
        .deposit(&project.id, &donator1, &token.address, &500i128);
    ctx.client
        .deposit(&project.id, &donator2, &token.address, &300i128);

    let updated = ctx.client.get_project(&project.id);
    assert_eq!(updated.donation_count, 2);
}

#[test]
fn test_donation_count_increments_for_same_donor_different_tokens() {
    let ctx = TestContext::new();

    // Setup manual project with 2 tokens
    let (token1, sac1) = ctx.create_token();
    let (token2, sac2) = ctx.create_token();
    let tokens =
        soroban_sdk::Vec::from_array(&ctx.env, [token1.address.clone(), token2.address.clone()]);
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &10_000,
        &ctx.dummy_proof(),
        &(ctx.env.ledger().timestamp() + 86400),
    );

    let donator = ctx.generate_address();
    sac1.mint(&donator, &1_000);
    sac2.mint(&donator, &1_000);

    ctx.client
        .deposit(&project.id, &donator, &token1.address, &500i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 1);

    ctx.client
        .deposit(&project.id, &donator, &token2.address, &300i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 2);
}

#[test]
fn test_donation_count_complex_scenario() {
    let ctx = TestContext::new();
    let (token1, sac1) = ctx.create_token();
    let (token2, sac2) = ctx.create_token();
    let tokens =
        soroban_sdk::Vec::from_array(&ctx.env, [token1.address.clone(), token2.address.clone()]);
    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &10_000,
        &ctx.dummy_proof(),
        &(ctx.env.ledger().timestamp() + 86400),
    );

    let donator1 = ctx.generate_address();
    let donator2 = ctx.generate_address();
    let donator3 = ctx.generate_address();

    sac1.mint(&donator1, &5_000);
    sac1.mint(&donator2, &5_000);
    sac1.mint(&donator3, &5_000);
    sac2.mint(&donator1, &5_000);
    sac2.mint(&donator2, &5_000);

    // Sequence of deposits
    ctx.client
        .deposit(&project.id, &donator1, &token1.address, &100i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 1);

    ctx.client
        .deposit(&project.id, &donator1, &token1.address, &100i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 1);

    ctx.client
        .deposit(&project.id, &donator2, &token1.address, &200i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 2);

    ctx.client
        .deposit(&project.id, &donator1, &token2.address, &150i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 3);

    ctx.client
        .deposit(&project.id, &donator3, &token1.address, &300i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 4);

    ctx.client
        .deposit(&project.id, &donator2, &token2.address, &250i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 5);

    ctx.client
        .deposit(&project.id, &donator2, &token2.address, &100i128);
    assert_eq!(ctx.client.get_project(&project.id).donation_count, 5);
}
