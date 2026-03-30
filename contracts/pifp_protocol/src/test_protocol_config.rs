extern crate std;

use crate::{test_utils::TestContext, Role};
use soroban_sdk::Address;

#[test]
fn test_update_protocol_config_success() {
    let ctx = TestContext::new();
    let recipient = ctx.generate_address();
    ctx.client.update_protocol_config(&ctx.admin, &recipient, &500);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_update_protocol_config_unauthorized() {
    let ctx = TestContext::new();
    let stranger = ctx.generate_address();
    let recipient = ctx.generate_address();
    ctx.client.update_protocol_config(&stranger, &recipient, &500);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #25)")]
fn test_update_protocol_config_invalid_bps() {
    let ctx = TestContext::new();
    let recipient = ctx.generate_address();
    ctx.client.update_protocol_config(&ctx.admin, &recipient, &1001);
}

#[test]
fn test_verify_and_release_with_fees() {
    let ctx = TestContext::new();
    let fee_recipient = ctx.generate_address();
    ctx.client.update_protocol_config(&ctx.admin, &fee_recipient, &500); // 5%

    let (project, token, sac) = ctx.setup_project(1000);
    let donor = ctx.generate_address();
    sac.mint(&donor, &1000i128);
    ctx.client.deposit(&project.id, &donor, &token.address, &1000i128);
    ctx.client.verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    // fee = 1000 * 500 / 10000 = 50; creator gets 950
    assert_eq!(token.balance(&fee_recipient), 50i128);
    assert_eq!(token.balance(&ctx.manager), 950i128);
    assert_eq!(token.balance(&ctx.client.address), 0i128);
}

#[test]
fn test_verify_and_release_zero_fee() {
    let ctx = TestContext::new();
    let fee_recipient = ctx.generate_address();
    ctx.client.update_protocol_config(&ctx.admin, &fee_recipient, &0);

    let (project, token, sac) = ctx.setup_project(1000);
    let donor = ctx.generate_address();
    sac.mint(&donor, &1000i128);
    ctx.client.deposit(&project.id, &donor, &token.address, &1000i128);
    ctx.client.verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

    assert_eq!(token.balance(&fee_recipient), 0i128);
    assert_eq!(token.balance(&ctx.manager), 1000i128);
}
