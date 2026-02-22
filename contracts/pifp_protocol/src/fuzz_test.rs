
extern crate std;
use std::vec::Vec;

use proptest::prelude::*;
use soroban_sdk::{testutils::Address as _, token, Address, BytesN, Env, Vec as SorobanVec};

use crate::invariants::*;
pub use crate::types::ProjectStatus;
pub use crate::Role;
use crate::{PifpProtocol, PifpProtocolClient};

// ── Helpers ─────────────────────────────────────────────────────────

fn setup_env() -> (Env, PifpProtocolClient<'static>, Address) {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(PifpProtocol, ());
    let client = PifpProtocolClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.init(&admin);
    (env, client, admin)
}

fn create_token<'a>(env: &Env, admin: &Address) -> token::Client<'a> {
    let addr = env.register_stellar_asset_contract_v2(admin.clone());
    token::Client::new(env, &addr.address())
}

// ── 1. Registration Fuzz Tests ──────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn fuzz_register_valid_goal(goal in 1i128..=1_000_000_000_000i128) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &[7u8; 32]);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &goal,
            &proof_hash,
            &deadline,
        );

        assert_all_project_invariants(&project);
        assert_eq!(project.goal, goal);
        assert_eq!(project.status, ProjectStatus::Funding);
    }

    #[test]
    fn fuzz_register_valid_deadline(offset in 1u64..=10_000_000u64) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &[8u8; 32]);
        let deadline = env.ledger().timestamp() + offset;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &100,
            &proof_hash,
            &deadline,
        );

        assert_all_project_invariants(&project);
        assert_eq!(project.deadline, deadline);
    }

    #[test]
    fn fuzz_register_random_proof_hash(hash_bytes in prop::array::uniform32(any::<u8>())) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &hash_bytes);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &1000,
            &proof_hash,
            &deadline,
        );

        assert_all_project_invariants(&project);
        assert_eq!(project.proof_hash, proof_hash);
    }
}

// ── 2. Deposit Fuzz Tests ───────────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn fuzz_deposit_single(amount in 1i128..=100_000i128) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token_client = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token_client.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &100_000,
            &proof_hash,
            &deadline,
        );

        let donator = Address::generate(&env);
        let sac = token::StellarAssetClient::new(&env, &token_client.address);
        sac.mint(&donator, &amount);

        let balance_before = client.get_balance(&project.id, &token_client.address);
        client.deposit(&project.id, &donator, &token_client.address, &amount);

        let balance_after = client.get_balance(&project.id, &token_client.address);
        assert_deposit_invariant(balance_before, balance_after, amount);
        
        let updated = client.get_project(&project.id);
        assert_all_project_invariants(&updated);
    }

    #[test]
    fn fuzz_deposit_multiple(
        amounts in prop::collection::vec(1i128..=10_000i128, 2..=8)
    ) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token_client = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &[2u8; 32]);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token_client.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &1_000_000,
            &proof_hash,
            &deadline,
        );

        let sac = token::StellarAssetClient::new(&env, &token_client.address);
        let mut expected_balance: i128 = 0;

        for amount in &amounts {
            let donator = Address::generate(&env);
            sac.mint(&donator, amount);

            let before = client.get_balance(&project.id, &token_client.address);
            client.deposit(&project.id, &donator, &token_client.address, amount);
            let after_balance = client.get_balance(&project.id, &token_client.address);

            assert_deposit_invariant(before, after_balance, *amount);
            
            let after = client.get_project(&project.id);
            assert_all_project_invariants(&after);

            expected_balance += amount;
        }

        let final_balance = client.get_balance(&project.id, &token_client.address);
        assert_eq!(final_balance, expected_balance);
    }
}

// ── 3. Verification Fuzz Tests ──────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn fuzz_verify_wrong_hash_always_fails(
        stored_bytes in prop::array::uniform32(any::<u8>()),
        submitted_bytes in prop::array::uniform32(any::<u8>()),
    ) {
        prop_assume!(stored_bytes != submitted_bytes);

        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &stored_bytes);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &500,
            &proof_hash,
            &deadline,
        );

        let oracle = Address::generate(&env);
        client.set_oracle(&admin, &oracle);

        let wrong_hash = BytesN::from_array(&env, &submitted_bytes);
        let result = client.try_verify_and_release(&oracle, &project.id, &wrong_hash);
        prop_assert!(result.is_err(), "verify_and_release should fail with wrong hash");
    }

    #[test]
    fn fuzz_verify_correct_hash_always_succeeds(
        hash_bytes in prop::array::uniform32(any::<u8>()),
    ) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &hash_bytes);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let project = client.register_project(
            &creator,
            &tokens,
            &500,
            &proof_hash,
            &deadline,
        );

        let oracle = Address::generate(&env);
        client.set_oracle(&admin, &oracle);

        client.verify_and_release(&oracle, &project.id, &proof_hash);

        let updated = client.get_project(&project.id);
        assert_valid_status_transition(&ProjectStatus::Funding, &updated.status);
        assert_eq!(updated.status, ProjectStatus::Completed);
    }
}

// ── 4. Sequential ID Invariant ──────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn fuzz_sequential_ids(n in 2u32..=10u32) {
        let (env, client, admin) = setup_env();
        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &[1u8; 32]);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let mut projects = Vec::new();
        for _ in 0..n {
            let creator = Address::generate(&env);
            client.grant_role(&admin, &creator, &Role::ProjectManager);

            let p = client.register_project(
                &creator,
                &tokens,
                &1000,
                &proof_hash,
                &deadline,
            );
            projects.push(p);
        }

        assert_sequential_ids(&projects);
    }
}

// ── 5. Immutability Invariant ───────────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn fuzz_immutability_after_deposit(amount in 1i128..=50_000i128) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token_client = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &[5u8; 32]);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token_client.address.clone());

        let original = client.register_project(
            &creator,
            &tokens,
            &100_000,
            &proof_hash,
            &deadline,
        );

        let donator = Address::generate(&env);
        let sac = token::StellarAssetClient::new(&env, &token_client.address);
        sac.mint(&donator, &amount);
        client.deposit(&original.id, &donator, &token_client.address, &amount);

        let after = client.get_project(&original.id);
        assert_project_immutable_fields(&original, &after);
    }

    #[test]
    fn fuzz_immutability_after_verify(
        hash_bytes in prop::array::uniform32(any::<u8>()),
    ) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &hash_bytes);
        let deadline = env.ledger().timestamp() + 86_400;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token.address.clone());

        let original = client.register_project(
            &creator,
            &tokens,
            &500,
            &proof_hash,
            &deadline,
        );

        let oracle = Address::generate(&env);
        client.set_oracle(&admin, &oracle);
        client.verify_and_release(&oracle, &original.id, &proof_hash);

        let after = client.get_project(&original.id);
        assert_project_immutable_fields(&original, &after);
    }
}

// ── 6. Full Lifecycle Stress Test ───────────────────────────────────

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn fuzz_full_lifecycle(
        goal in 100i128..=1_000_000i128,
        deposit_amounts in prop::collection::vec(1i128..=10_000i128, 1..=5),
        hash_bytes in prop::array::uniform32(any::<u8>()),
        deadline_offset in 1000u64..=10_000_000u64,
    ) {
        let (env, client, admin) = setup_env();
        let creator = Address::generate(&env);
        client.grant_role(&admin, &creator, &Role::ProjectManager);

        let token_admin = Address::generate(&env);
        let token_client = create_token(&env, &token_admin);
        let proof_hash = BytesN::from_array(&env, &hash_bytes);
        let deadline = env.ledger().timestamp() + deadline_offset;

        let mut tokens = SorobanVec::new(&env);
        tokens.push_back(token_client.address.clone());

        // Phase 1: Register project.
        let project = client.register_project(
            &creator,
            &tokens,
            &goal,
            &proof_hash,
            &deadline,
        );
        assert_all_project_invariants(&project);
        assert_eq!(project.status, ProjectStatus::Funding);

        // Phase 2: Multiple deposits.
        let sac = token::StellarAssetClient::new(&env, &token_client.address);
        let mut total_deposited: i128 = 0;

        for amount in &deposit_amounts {
            let donator = Address::generate(&env);
            sac.mint(&donator, amount);

            let before_balance = client.get_balance(&project.id, &token_client.address);
            client.deposit(&project.id, &donator, &token_client.address, amount);
            let after_balance = client.get_balance(&project.id, &token_client.address);

            assert_deposit_invariant(before_balance, after_balance, *amount);
            
            let after = client.get_project(&project.id);
            assert_project_immutable_fields(&project, &after);
            assert_all_project_invariants(&after);

            total_deposited += amount;
        }

        let final_balance = client.get_balance(&project.id, &token_client.address);
        assert_eq!(final_balance, total_deposited);

        // Phase 3: Oracle verification.
        let oracle = Address::generate(&env);
        client.set_oracle(&admin, &oracle);
        client.verify_and_release(&oracle, &project.id, &proof_hash);

        let final_project = client.get_project(&project.id);
        assert_valid_status_transition(&ProjectStatus::Funding, &final_project.status);
        assert_project_immutable_fields(&project, &final_project);
        assert_eq!(final_project.status, ProjectStatus::Completed);
        
        // Balance should be unchanged after verification.
        let post_verify_balance = client.get_balance(&project.id, &token_client.address);
        assert_eq!(post_verify_balance, total_deposited);

        // Phase 4: Double-verify should fail.
        let result = client.try_verify_and_release(&oracle, &project.id, &proof_hash);
        prop_assert!(result.is_err(), "double verification should fail");
    }
}
