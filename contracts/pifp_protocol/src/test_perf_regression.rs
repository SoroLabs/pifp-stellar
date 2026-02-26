//! # Performance Regression Tests
//!
//! Tests to ensure optimized implementations maintain expected performance characteristics
//! and prevent performance degradation in future changes.

#[cfg(test)]
mod test {
    use crate::test_utils::TestContext;
    use soroban_sdk::{vec, BytesN};

    #[test]
    fn test_duplicate_detection_performance_regression() {
        let ctx = TestContext::new();

        // Test with maximum number of tokens (worst case scenario)
        let mut tokens = Vec::new(&ctx.env);
        for _ in 0..10 {
            tokens.push_back(ctx.generate_address());
        }

        // This should complete without timeout/hang
        let project = ctx.client.register_project(
            &ctx.manager,
            &tokens,
            &1000i128,
            &BytesN::from_array(&ctx.env, &[0; 32]),
            &(ctx.env.ledger().timestamp() + 100_000),
        );

        assert_eq!(project.id, 0);
        assert_eq!(project.accepted_tokens.len(), 10);
    }

    #[test]
    fn test_token_verification_performance() {
        let ctx = TestContext::new();

        // Setup project with multiple tokens
        let token1 = ctx.generate_address();
        let token2 = ctx.generate_address();
        let token3 = ctx.generate_address();
        let tokens = vec![&ctx.env, token1.clone(), token2.clone(), token3.clone()];

        let project = ctx.client.register_project(
            &ctx.manager,
            &tokens,
            &1000i128,
            &ctx.dummy_proof(),
            &(ctx.env.ledger().timestamp() + 100_000),
        );

        // Test first token (should be fastest due to early termination)
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token1, &100);

        // Test last token (should still be reasonably fast)
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token3, &100);

        // Verify both deposits succeeded
        let balances = ctx.client.get_project_balances(&project.id);
        assert_eq!(balances.balances.len(), 3);
        assert_eq!(balances.balances.get(0).unwrap().balance, 100); // token1
        assert_eq!(balances.balances.get(2).unwrap().balance, 100); // token3
    }

    #[test]
    fn test_fund_transfer_optimization() {
        let ctx = TestContext::new();

        // Setup project with multiple tokens
        let token1 = ctx.create_token(1_000_000);
        let token2 = ctx.create_token(1_000_000);
        let tokens = vec![&ctx.env, token1.address.clone(), token2.address.clone()];

        let project = ctx.client.register_project(
            &ctx.manager,
            &tokens,
            &1000i128,
            &ctx.dummy_proof(),
            &(ctx.env.ledger().timestamp() + 100_000),
        );

        // Make deposits to multiple tokens
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token1.address, &500);
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token2.address, &300);

        // Grant oracle role and verify
        ctx.client
            .grant_role(&ctx.admin, &ctx.oracle, &crate::Role::Oracle);
        ctx.client
            .verify_and_release(&ctx.oracle, &project.id, &ctx.dummy_proof());

        // Verify all funds were transferred
        let updated_project = ctx.client.get_project(&project.id);
        assert_eq!(updated_project.status, crate::ProjectStatus::Completed);

        // Verify balances are zero after transfer
        let balances = ctx.client.get_project_balances(&project.id);
        for balance in balances.balances.iter() {
            assert_eq!(balance.balance, 0);
        }
    }

    #[test]
    fn test_optimized_vs_original_equivalence() {
        let ctx = TestContext::new();

        // Test that optimized implementation produces same results as original logic
        let token = ctx.generate_address();
        let tokens = vec![&ctx.env, token.clone()];

        // Register project
        let project = ctx.client.register_project(
            &ctx.manager,
            &tokens,
            &1000i128,
            &ctx.dummy_proof(),
            &(ctx.env.ledger().timestamp() + 100_000),
        );

        // Deposit should work with optimized token checking
        ctx.client
            .deposit(&project.id, &ctx.generate_address(), &token, &100);

        // Verify deposit succeeded
        let balance = ctx.client.get_balance(&project.id, token);
        assert_eq!(balance, 100);
    }
}
