extern crate std;

use soroban_sdk::{testutils::Address as _, Address, BytesN, Env};

use crate::types::ProjectStatus;
use crate::{PifpProtocol, PifpProtocolClient};

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();

    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    (env, client)
}

// ── Project Registry Tests ──────────────────────────────────────────

#[test]
fn test_register_project_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
    let goal: i128 = 1_000;
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &goal, &proof_hash, &deadline);

    assert_eq!(project.id, 0);
    assert_eq!(project.creator, creator);
    assert_eq!(project.goal, goal);
    assert_eq!(project.balance, 0);
    assert_eq!(project.proof_hash, proof_hash);
    assert_eq!(project.deadline, deadline);
    assert_eq!(project.status, ProjectStatus::Funding);
}

#[test]
fn test_register_second_project_unique_ids() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[2u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let p1 = client.register_project(&creator, &500, &proof_hash, &deadline);
    let p2 = client.register_project(&creator, &700, &proof_hash, &deadline);

    assert_eq!(p1.id, 0);
    assert_eq!(p2.id, 1);
}

#[test]
#[should_panic(expected = "goal must be positive")]
fn test_register_project_invalid_goal() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[3u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    client.register_project(&creator, &0, &proof_hash, &deadline);
}

#[test]
#[should_panic(expected = "deadline must be in the future")]
fn test_register_project_invalid_deadline() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[4u8; 32]);
    let deadline: u64 = 0;

    client.register_project(&creator, &100, &proof_hash, &deadline);
}

#[test]
fn test_get_project_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[5u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let registered = client.register_project(&creator, &999, &proof_hash, &deadline);
    let retrieved = client.get_project(&registered.id);

    assert_eq!(registered, retrieved);
}

#[test]
#[should_panic(expected = "project not found")]
fn test_get_project_not_found() {
    let (_env, client) = setup();

    client.get_project(&42);
}

// ── ZK-Proof Verification Tests ─────────────────────────────────────

#[test]
fn test_set_oracle() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);

    client.set_oracle(&admin, &oracle);

    // Verify by using the oracle for verification (indirectly tested).
    // Direct storage read is not possible from the test client,
    // so we verify via a successful verify_and_release below.
}

#[test]
fn test_verify_and_release_success() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &500, &proof_hash, &deadline);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&admin, &oracle);

    // Oracle verifies with the correct proof hash.
    client.verify_and_release(&project.id, &proof_hash);

    // Check project status is now Completed.
    let updated = client.get_project(&project.id);
    assert_eq!(updated.status, ProjectStatus::Completed);
}

#[test]
#[should_panic(expected = "proof verification failed: hash mismatch")]
fn test_verify_wrong_hash() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let wrong_hash = BytesN::from_array(&env, &[99u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &500, &proof_hash, &deadline);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&admin, &oracle);

    client.verify_and_release(&project.id, &wrong_hash);
}

#[test]
#[should_panic(expected = "project already completed")]
fn test_verify_already_completed() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &500, &proof_hash, &deadline);

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&admin, &oracle);

    // First verification succeeds.
    client.verify_and_release(&project.id, &proof_hash);

    // Second verification should fail.
    client.verify_and_release(&project.id, &proof_hash);
}

#[test]
#[should_panic(expected = "project not found")]
fn test_verify_nonexistent_project() {
    let (env, client) = setup();

    let admin = Address::generate(&env);
    let oracle = Address::generate(&env);
    client.set_oracle(&admin, &oracle);

    let fake_hash = BytesN::from_array(&env, &[0u8; 32]);
    client.verify_and_release(&999, &fake_hash);
}

#[test]
#[should_panic(expected = "oracle not set")]
fn test_verify_without_oracle_set() {
    let (env, client) = setup();

    let creator = Address::generate(&env);
    let proof_hash = BytesN::from_array(&env, &[10u8; 32]);
    let deadline: u64 = env.ledger().timestamp() + 86_400;

    let project = client.register_project(&creator, &500, &proof_hash, &deadline);

    // No oracle set — should panic.
    client.verify_and_release(&project.id, &proof_hash);
}
