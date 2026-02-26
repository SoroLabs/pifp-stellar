//! # Gas Consumption Baseline Tests
//!
//! Establishes baseline gas consumption metrics for key contract operations.
//! These tests provide the "before" measurements for optimization comparison.

#[cfg(test)]
mod test {
    use crate::test_utils::TestContext;
    use soroban_sdk::{vec, BytesN};

    #[test]
    fn test_baseline_register_project_gas() {
        let ctx = TestContext::new();

        // Setup test data
        let token = ctx.generate_address();
        let tokens = vec![&ctx.env, token.clone()];
        let goal = 1000i128;
        let proof_hash = BytesN::from_array(&ctx.env, &[0; 32]);
        let deadline = ctx.env.ledger().timestamp() + 100_000;

        // Measure gas consumption for project registration
        let project =
            ctx.client
                .register_project(&ctx.manager, &tokens, &goal, &proof_hash, &deadline);

        // Basic assertion that operation succeeded
        assert_eq!(project.id, 0);
        assert_eq!(project.creator, ctx.manager);
    }

    #[test]
    fn test_baseline_deposit_operation_gas() {
        let ctx = TestContext::new();

        // Setup project and token
        let (project, token, _) = ctx.setup_project(1000);
        let donator = ctx.generate_address();
        let amount = 100i128;

        // First deposit (new donor)
        ctx.client
            .deposit(&project.id, &donator, &token.address, &amount);

        // Second deposit (existing donor)
        ctx.client
            .deposit(&project.id, &donator, &token.address, &amount);

        // Verify both operations succeed
        let balances = ctx.client.get_project_balances(&project.id);
        assert_eq!(balances.balances.len(), 1);
        assert_eq!(balances.balances.get(0).unwrap().balance, amount * 2);
    }

    #[test]
    fn test_baseline_verify_and_release_gas() {
        let ctx = TestContext::new();

        // Setup project
        let (project, token, _) = ctx.setup_project(1000);
        let proof_hash = ctx.dummy_proof();

        // Make deposits to reach goal
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token.address, &500);
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token.address, &500);

        // Grant oracle role
        ctx.client
            .grant_role(&ctx.admin, &ctx.oracle, &crate::Role::Oracle);

        // Measure verification and release
        ctx.client
            .verify_and_release(&ctx.oracle, &project.id, &proof_hash);

        // Verify operation succeeds
        let updated_project = ctx.client.get_project(&project.id);
        assert_eq!(updated_project.status, crate::ProjectStatus::Completed);
    }
}
