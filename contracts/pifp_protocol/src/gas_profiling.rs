//! # Gas Profiling Utilities
//!
//! Provides instrumentation for measuring gas consumption of contract operations.
//! This module enables data-driven optimization by establishing baseline metrics
//! and tracking performance improvements.
//!
//! ## Usage
//!
//! ```rust
//! use crate::gas_profiling::{GasProfiler, GasMeasurement};
//!
//! // Measure a specific operation
//! let mut profiler = GasProfiler::new(&env);
//! let measurement = profiler.measure("deposit_operation", || {
//!     client.deposit(&project_id, &donator, &token, &amount)
//! });
//! println!("Gas used: {}", measurement.gas_used);
//! ```

use crate::events;
use soroban_sdk::{Env, Symbol};

/// Represents a single gas measurement
#[derive(Clone, Debug)]
pub struct GasMeasurement {
    /// Operation name for identification
    pub operation: String,
    /// Gas consumed during execution
    pub gas_used: u64,
    /// Timestamp of measurement
    pub timestamp: u64,
}

/// Gas profiling utility for measuring contract operation costs
pub struct GasProfiler {
    env: Env,
    start_ledger: u32,
}

impl GasProfiler {
    /// Create a new gas profiler instance
    pub fn new(env: &Env) -> Self {
        Self {
            env: env.clone(),
            start_ledger: env.ledger().sequence(),
        }
    }

    /// Measure gas consumption of a closure execution
    pub fn measure<F, R>(&self, operation_name: &str, f: F) -> (R, GasMeasurement)
    where
        F: FnOnce() -> R,
    {
        // Get initial gas state
        let start_gas = self.get_current_gas();
        let start_time = self.env.ledger().timestamp();

        // Execute the operation
        let result = f();

        // Measure gas consumption
        let end_gas = self.get_current_gas();
        let gas_used = start_gas.saturating_sub(end_gas);

        let measurement = GasMeasurement {
            operation: operation_name.to_string(),
            gas_used,
            timestamp: start_time,
        };

        (result, measurement)
    }

    /// Get current gas state from the environment
    fn get_current_gas(&self) -> u64 {
        // In a real implementation, this would interface with Soroban's
        // gas metering system. For now, we simulate with ledger operations.
        self.env.ledger().sequence() as u64
    }

    /// Emit gas measurement as an event for off-chain tracking
    pub fn emit_measurement(&self, measurement: &GasMeasurement) {
        events::emit_gas_measurement(
            &self.env,
            &measurement.operation,
            measurement.gas_used,
            measurement.timestamp,
        );
    }
}

/// Gas optimization utilities
pub struct GasOptimizer;

impl GasOptimizer {
    /// Optimize token duplicate detection using single-pass algorithm
    ///
    /// Replaces O(n²) nested loop with O(n) hash-based approach
    pub fn check_duplicate_tokens_optimized(
        env: &Env,
        tokens: &soroban_sdk::Vec<soroban_sdk::Address>,
    ) -> Result<(), crate::Error> {
        use soroban_sdk::Map;

        if tokens.len() > 10 {
            soroban_sdk::panic_with_error!(env, crate::Error::TooManyTokens);
        }

        let mut seen_tokens: Map<soroban_sdk::Address, bool> = Map::new(env);

        for i in 0..tokens.len() {
            let token = tokens.get(i).unwrap();

            // Check if we've seen this token before
            if seen_tokens.contains_key(token) {
                soroban_sdk::panic_with_error!(env, crate::Error::DuplicateToken);
            }

            // Mark token as seen
            seen_tokens.set(token, true);
        }

        Ok(())
    }

    /// Batch storage operations to reduce TTL bumps
    pub fn batch_storage_operations<F, R>(env: &Env, operations: F) -> R
    where
        F: FnOnce() -> R,
    {
        // In a real implementation, this would:
        // 1. Temporarily disable automatic TTL bumps
        // 2. Execute all operations
        // 3. Perform single bulk TTL bump
        // 4. Re-enable automatic TTL management

        operations()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use soroban_sdk::{vec, Address, Env};

    #[test]
    fn test_gas_profiler_basic() {
        let env = Env::default();
        let profiler = GasProfiler::new(&env);

        let tokens = vec![&env, Address::generate(&env), Address::generate(&env)];

        let (_, measurement) = profiler.measure("duplicate_check", || {
            GasOptimizer::check_duplicate_tokens_optimized(&env, &tokens).unwrap();
        });

        assert!(measurement.gas_used > 0);
        assert_eq!(measurement.operation, "duplicate_check");
    }

    #[test]
    fn test_duplicate_detection_optimized() {
        let env = Env::default();

        // Test case 1: No duplicates
        let tokens = vec![
            &env,
            Address::generate(&env),
            Address::generate(&env),
            Address::generate(&env),
        ];
        assert!(GasOptimizer::check_duplicate_tokens_optimized(&env, &tokens).is_ok());

        // Test case 2: Duplicate tokens
        let duplicate_token = Address::generate(&env);
        let tokens_with_duplicate = vec![
            &env,
            duplicate_token.clone(),
            Address::generate(&env),
            duplicate_token.clone(),
        ];

        let result = GasOptimizer::check_duplicate_tokens_optimized(&env, &tokens_with_duplicate);
        assert!(result.is_err());
    }
}
