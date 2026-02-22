extern crate std;
 
use soroban_sdk::{
    testutils::{Address as _, Ledger},
    Address, BytesN, Env,
};

use crate::{PifpProtocol, PifpProtocolClient, ProjectStatus};

fn setup() -> (Env, PifpProtocolClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    
    let admin = Address::generate(&env);
    client.init(&admin);
    
    (env, client, admin)
}

fn dummy_proof(env: &Env) -> BytesN<32> {
    BytesN::from_array(env, &[0xabu8; 32])
}

#[test]
fn test_expire_project_success() {
    let (env, client, admin) = setup();
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1000;
    
    let project = client.register_project(
        &admin,
        &soroban_sdk::vec![&env, token],
        &1000i128,
        &dummy_proof(&env),
        &deadline,
    );
    
    assert_eq!(project.status, ProjectStatus::Funding);
    
    // Jump forward in time
    env.ledger().set_timestamp(deadline + 1);
    
    client.expire_project(&project.id);
    
    let expired_project = client.get_project(&project.id);
    assert_eq!(expired_project.status, ProjectStatus::Expired);
}

#[test]
#[should_panic(expected = "project has not expired yet")]
fn test_expire_before_deadline_panics() {
    let (env, client, admin) = setup();
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1000;
    
    let project = client.register_project(
        &admin,
        &soroban_sdk::vec![&env, token],
        &1000i128,
        &dummy_proof(&env),
        &deadline,
    );
    
    // Attempt to expire before deadline
    client.expire_project(&project.id);
}

#[test]
#[should_panic(expected = "invalid transition: only funding projects can expire")]
fn test_expire_wrong_status_panics() {
    let (env, client, admin) = setup();
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1000;
    
    let project = client.register_project(
        &admin,
        &soroban_sdk::vec![&env, token],
        &1000i128,
        &dummy_proof(&env),
        &deadline,
    );
    
    // Forcing an Active status would involve a deposit, but easier is just use a mock or verify via other means.
    // However, the check is explicitly for Status::Funding.
    // Since I can't easily reach Active without full token setup in this isolated test, 
    // I'll at least verify the guard is there.
    
    // Verify it fails if we call it twice (since first time sets it to Expired)
    env.ledger().set_timestamp(deadline + 1);
    client.expire_project(&project.id);
    client.expire_project(&project.id); // Should panic here
}

#[test]
#[should_panic(expected = "invalid transition: only funding projects can expire")]
fn test_expire_completed_project_panics() {
    let (env, client, admin) = setup();
    let token = Address::generate(&env);
    let deadline = env.ledger().timestamp() + 1000;
    
    let oracle = Address::generate(&env);
    client.grant_role(&admin, &oracle, &crate::Role::Oracle);
    
    let proof = dummy_proof(&env);
    let project = client.register_project(
        &admin,
        &soroban_sdk::vec![&env, token],
        &1000i128,
        &proof,
        &deadline,
    );
    
    // Move to Completed
    client.verify_and_release(&oracle, &project.id, &proof);
    
    // Attempt to expire
    env.ledger().set_timestamp(deadline + 1);
    client.expire_project(&project.id);
}
