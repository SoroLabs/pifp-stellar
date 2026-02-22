extern crate std;

use soroban_sdk::{
    symbol_short,
    testutils::{Address as _, Events},
    token, vec, Address, BytesN, Env, IntoVal, TryIntoVal,
};

use crate::events::{ProjectCreated, ProjectFunded, ProjectVerified};
use crate::{PifpProtocol, PifpProtocolClient, Role};

fn setup() -> (Env, PifpProtocolClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    (env, client)
}

fn setup_with_init() -> (Env, PifpProtocolClient<'static>, Address) {
    let (env, client) = setup();
    let super_admin = Address::generate(&env);
    client.init(&super_admin);
    (env, client, super_admin)
}

fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &addr.address())
}

#[test]
fn test_project_created_event() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let goal = 5000i128;
    let proof_hash = BytesN::from_array(&env, &[0xabu8; 32]);
    let deadline = env.ledger().timestamp() + 86400;

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);

    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(&creator, &tokens, &goal, &proof_hash, &deadline);

    let all_events = env.events().all();
    let last_event = all_events.last().expect("No events found");

    // Topic: (symbol_short!("created"), project_id)
    assert_eq!(last_event.0, client.address);
    let expected_topics = vec![
        &env,
        symbol_short!("created").into_val(&env),
        project.id.into_val(&env),
    ];
    assert_eq!(last_event.1, expected_topics);

    // Data: ProjectCreated struct
    let event_data: ProjectCreated = last_event.2.try_into_val(&env).unwrap();
    assert_eq!(
        event_data,
        ProjectCreated {
            project_id: project.id,
            creator: creator.clone(),
            token: token.address.clone(),
            goal,
        }
    );
}

#[test]
fn test_project_funded_event() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let donator = Address::generate(&env);
    let amount = 1000i128;

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &10000,
        &BytesN::from_array(&env, &[0u8; 32]),
        &(env.ledger().timestamp() + 86400),
    );

    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &amount);

    client.deposit(&project.id, &donator, &token.address, &amount);

    let all_events = env.events().all();
    let last_event = all_events.last().expect("No events found");

    // Topic: (symbol_short!("funded"), project_id)
    assert_eq!(last_event.0, client.address);
    let expected_topics = vec![
        &env,
        symbol_short!("funded").into_val(&env),
        project.id.into_val(&env),
    ];
    assert_eq!(last_event.1, expected_topics);

    // Data: ProjectFunded struct
    let event_data: ProjectFunded = last_event.2.try_into_val(&env).unwrap();
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
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let oracle = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let token = create_token(&env, &token_admin);
    let proof_hash = BytesN::from_array(&env, &[0xabu8; 32]);

    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    client.set_oracle(&super_admin, &oracle);

    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &1000,
        &proof_hash,
        &(env.ledger().timestamp() + 86400),
    );

    client.verify_and_release(&oracle, &project.id, &proof_hash);

    let all_events = env.events().all();
    let last_event = all_events.last().expect("No events found");

    // Topic: (symbol_short!("verified"), project_id)
    assert_eq!(last_event.0, client.address);
    let expected_topics = vec![
        &env,
        symbol_short!("verified").into_val(&env),
        project.id.into_val(&env),
    ];
    assert_eq!(last_event.1, expected_topics);

    // Data: ProjectVerified struct
    let event_data: ProjectVerified = last_event.2.try_into_val(&env).unwrap();
    assert_eq!(
        event_data,
        ProjectVerified {
            project_id: project.id,
            oracle: oracle.clone(),
            proof_hash: proof_hash.clone(),
        }
    );
}

/// Integration test: verify_and_release transfers funds to creator
#[test]
fn test_funds_released_to_creator() {
    let (env, client, super_admin) = setup_with_init();
    let creator = Address::generate(&env);
    let oracle = Address::generate(&env);
    let token_admin = Address::generate(&env);
    let donator = Address::generate(&env);
    let deposit_amount = 1000i128;
    let proof_hash = BytesN::from_array(&env, &[0xabu8; 32]);

    // Create a mock token
    let token = create_token(&env, &token_admin);

    // Grant roles
    client.grant_role(&super_admin, &creator, &Role::ProjectManager);
    client.set_oracle(&super_admin, &oracle);

    // Register project
    let tokens = soroban_sdk::vec![&env, token.address.clone()];
    let project = client.register_project(
        &creator,
        &tokens,
        &5000,
        &proof_hash,
        &(env.ledger().timestamp() + 86400),
    );

    // Mint tokens to donator and deposit
    let token_sac = token::StellarAssetClient::new(&env, &token.address);
    token_sac.mint(&donator, &deposit_amount);
    client.deposit(&project.id, &donator, &token.address, &deposit_amount);

    // Verify and release - this should transfer funds to creator
    client.verify_and_release(&oracle, &project.id, &proof_hash);

    // Check creator received the funds
    let creator_token_client = token::Client::new(&env, &token.address);
    let creator_balance = creator_token_client.balance(&creator);
    assert_eq!(creator_balance, deposit_amount, "Creator should receive the deposited funds");

    // Check contract no longer has the funds
    let contract_balance = creator_token_client.balance(&client.address);
    assert_eq!(contract_balance, 0, "Contract should have zero balance after release");
}
