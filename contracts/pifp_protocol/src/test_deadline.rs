use crate::test_utils::{create_token, dummy_metadata_uri, dummy_proof, setup_test};
use crate::Role;
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, Vec,
};

#[test]
fn test_extend_deadline_success() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    // Register project manager
    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let now = 1000;
    env.ledger().set_timestamp(now);
    let deadline = now + 10000;

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &deadline,
        &false,
        &0u32,
    );

    let new_deadline = deadline + 5000;
    client.extend_deadline(&creator, &project.id, &new_deadline);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.deadline, new_deadline);
}

#[test]
fn test_extend_deadline_by_admin() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let now = 1000;
    env.ledger().set_timestamp(now);
    let deadline = now + 10000;

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &deadline,
        &false,
        &0u32,
    );

    let new_deadline = deadline + 5000;
    client.extend_deadline(&admin, &project.id, &new_deadline);

    let updated_project = client.get_project(&project.id);
    assert_eq!(updated_project.deadline, new_deadline);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #6)")]
fn test_extend_deadline_unauthorized() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let stranger = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &(env.ledger().timestamp() + 10000),
        &false,
        &0u32,
    );

    client.extend_deadline(&stranger, &project.id, &(env.ledger().timestamp() + 15000));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #13)")]
fn test_extend_deadline_backwards() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let deadline = env.ledger().timestamp() + 10000;
    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &deadline,
        &false,
        &0u32,
    );

    // New deadline same as or earlier than current is Error::InvalidDeadline (13)
    client.extend_deadline(&creator, &project.id, &deadline);
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #14)")]
fn test_extend_deadline_expired() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let now = 1000;
    env.ledger().set_timestamp(now);
    let deadline = now + 10000;

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &deadline,
        &false,
        &0u32,
    );

    // Fast forward past deadline
    env.ledger().set_timestamp(deadline + 1);

    client.extend_deadline(&creator, &project.id, &(deadline + 5000));
}

#[test]
#[should_panic(expected = "HostError: Error(Contract, #24)")]
fn test_extend_deadline_too_long() {
    let (env, client, admin) = setup_test();
    let creator = Address::generate(&env);
    let token = create_token(&env, &admin);
    let accepted_tokens = Vec::from_array(&env, [token.address.clone()]);

    client.grant_role(&admin, &creator, &Role::ProjectManager);

    let now = 1000;
    env.ledger().set_timestamp(now);
    let deadline = now + 10000;

    let project = client.register_project(
        &creator,
        &accepted_tokens,
        &1000,
        &dummy_proof(&env),
        &dummy_metadata_uri(&env),
        &deadline,
        &false,
        &0u32,
    );

    // 1 year + 1 second
    let too_late = now + 31_536_000 + 1;
    client.extend_deadline(&creator, &project.id, &too_late);
}
