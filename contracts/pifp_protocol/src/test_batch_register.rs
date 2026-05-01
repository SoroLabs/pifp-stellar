extern crate std;

use soroban_sdk::{vec, Address, Bytes, BytesN, Vec};

use crate::{types::{Milestone, ProjectRegistrationRequest}, test_utils::TestContext};

#[test]
fn test_batch_register_projects_success() {
    let ctx = TestContext::new();
    let env = &ctx.env;
    let token_a = ctx.create_token().0.address.clone();
    let token_b = ctx.create_token().0.address.clone();

    let proof_hash = ctx.dummy_proof();
    let metadata_uri = ctx.dummy_metadata_uri();
    let deadline = env.ledger().timestamp() + 86_400;

    let mut milestones_a = Vec::new(env);
    milestones_a.push_back(Milestone {
        label: BytesN::from_array(env, &[0u8; 32]),
        amount_bps: 10000,
        proof_hash: proof_hash.clone(),
    });

    let mut milestones_b = Vec::new(env);
    milestones_b.push_back(Milestone {
        label: BytesN::from_array(env, &[1u8; 32]),
        amount_bps: 10000,
        proof_hash: proof_hash.clone(),
    });

    let mut request_a = ProjectRegistrationRequest {
        accepted_tokens: Vec::from_array(env, [token_a.clone()]),
        goal: 5_000,
        proof_hash: proof_hash.clone(),
        metadata_uri: metadata_uri.clone(),
        deadline,
        is_private: false,
        milestones: milestones_a,
        categories: 0,
        authorized_oracles: Vec::new(env),
        threshold: 0,
    };

    let mut request_b = ProjectRegistrationRequest {
        accepted_tokens: Vec::from_array(env, [token_b.clone()]),
        goal: 10_000,
        proof_hash: proof_hash.clone(),
        metadata_uri: metadata_uri.clone(),
        deadline,
        is_private: false,
        milestones: milestones_b,
        categories: 0,
        authorized_oracles: Vec::new(env),
        threshold: 0,
    };

    let mut requests = Vec::new(env);
    requests.push_back(request_a);
    requests.push_back(request_b);

    let projects = ctx.client.batch_register_projects(&ctx.manager, &requests);
    assert_eq!(projects.len(), 2);
    assert_eq!(projects.get(0).unwrap().id, 0);
    assert_eq!(projects.get(1).unwrap().id, 1);

    let events = env.events().all();
    assert_eq!(events.len(), 2);
}

#[test]
fn test_batch_register_projects_atomicity() {
    let ctx = TestContext::new();
    let env = &ctx.env;
    let token = ctx.create_token().0.address.clone();

    let proof_hash = ctx.dummy_proof();
    let metadata_uri = ctx.dummy_metadata_uri();
    let deadline = env.ledger().timestamp() + 86_400;

    let mut milestones = Vec::new(env);
    milestones.push_back(Milestone {
        label: BytesN::from_array(env, &[0u8; 32]),
        amount_bps: 10000,
        proof_hash: proof_hash.clone(),
    });

    let valid_request = ProjectRegistrationRequest {
        accepted_tokens: Vec::from_array(env, [token.clone()]),
        goal: 5_000,
        proof_hash: proof_hash.clone(),
        metadata_uri: metadata_uri.clone(),
        deadline,
        is_private: false,
        milestones: milestones.clone(),
        categories: 0,
        authorized_oracles: Vec::new(env),
        threshold: 0,
    };

    let invalid_request = ProjectRegistrationRequest {
        accepted_tokens: Vec::from_array(env, [token]),
        goal: 0,
        proof_hash: proof_hash.clone(),
        metadata_uri: metadata_uri.clone(),
        deadline,
        is_private: false,
        milestones,
        categories: 0,
        authorized_oracles: Vec::new(env),
        threshold: 0,
    };

    let mut requests = Vec::new(env);
    requests.push_back(valid_request);
    requests.push_back(invalid_request);

    let result = std::panic::catch_unwind(|| {
        ctx.client.batch_register_projects(&ctx.manager, &requests);
    });

    assert!(result.is_err());
    assert_eq!(env.events().all().len(), 0);

    let mut tokens = Vec::new(env);
    tokens.push_back(ctx.create_token().0.address.clone());
    let mut milestones_recovery = Vec::new(env);
    milestones_recovery.push_back(Milestone {
        label: BytesN::from_array(env, &[2u8; 32]),
        amount_bps: 10000,
        proof_hash: proof_hash.clone(),
    });

    let project = ctx.client.register_project(
        &ctx.manager,
        &tokens,
        &5_000,
        &proof_hash,
        &metadata_uri,
        &deadline,
        &false,
        &milestones_recovery,
        &0u32,
        &Vec::new(env),
        &0u32,
    );

    assert_eq!(project.id, 0);
}
