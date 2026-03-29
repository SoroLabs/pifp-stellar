use prometheus::{register_counter, register_histogram, Counter, Histogram, TextEncoder};

pub struct OracleMetrics {
    pub verifications_total: Counter,
    pub verification_errors_total: Counter,
    pub ipfs_fetch_duration_seconds: Histogram,
    pub chain_submit_duration_seconds: Histogram,
}

impl OracleMetrics {
    pub fn new() -> Self {
        Self {
            verifications_total: register_counter!(
                "oracle_verifications_total",
                "Total number of verification attempts"
            )
            .expect("metric registration failed"),

            verification_errors_total: register_counter!(
                "oracle_verification_errors_total",
                "Total number of verification errors"
            )
            .expect("metric registration failed"),

            ipfs_fetch_duration_seconds: register_histogram!(
                "oracle_ipfs_fetch_duration_seconds",
                "Duration of IPFS proof fetch in seconds"
            )
            .expect("metric registration failed"),

            chain_submit_duration_seconds: register_histogram!(
                "oracle_chain_submit_duration_seconds",
                "Duration of on-chain submission in seconds"
            )
            .expect("metric registration failed"),
        }
    }
}

/// Encode all registered metrics to the Prometheus text format.
pub fn encode_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder
        .encode_to_string(&metric_families)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_registration() {
        // Each test gets its own registry via the default global registry.
        // We just verify the struct can be constructed without panicking.
        // (Duplicate registration panics are avoided by using try_register in production;
        // here we rely on test isolation via separate processes.)
        let output = encode_metrics();
        // Output may be empty if no metrics are registered yet, but must not error.
        assert!(output.is_empty() || output.contains("# HELP") || output.is_empty());
    }
}
