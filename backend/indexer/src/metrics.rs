//! Prometheus metrics for the PIFP event indexer.
//!
//! Exposes four metrics:
//! - `block_processing_time`  — gauge: seconds taken to process a single ledger poll.
//! - `total_events_indexed`   — counter: cumulative Stellar events returned by successful RPC polls.
//! - `rpc_latency`            — histogram: full round-trip time (send + body parse) per RPC call.
//! - `rpc_errors_total`       — counter: number of RPC calls that resulted in an error.

use lazy_static::lazy_static;
use prometheus::{
    register_counter, register_gauge, register_histogram, Counter, Gauge, Histogram,
    HistogramOpts, Opts, TextEncoder,
};

lazy_static! {
    /// Seconds taken to process a single ledger poll (decode + DB write).
    pub static ref BLOCK_PROCESSING_TIME: Gauge = register_gauge!(
        Opts::new(
            "block_processing_time",
            "Seconds taken to process a single ledger poll"
        )
    )
    .expect("failed to register block_processing_time gauge");

    /// Total Stellar events returned by successful RPC polls since startup.
    pub static ref TOTAL_EVENTS_INDEXED: Counter = register_counter!(
        Opts::new(
            "total_events_indexed",
            "Total count of Stellar events returned by successful RPC polls"
        )
    )
    .expect("failed to register total_events_indexed counter");

    /// Full round-trip latency (seconds) per RPC call, including body parsing.
    pub static ref RPC_LATENCY: Histogram = register_histogram!(
        HistogramOpts::new(
            "rpc_latency",
            "Full round-trip response time in seconds from the Stellar RPC node"
        )
        .buckets(vec![0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0])
    )
    .expect("failed to register rpc_latency histogram");

    /// Total number of RPC calls that resulted in a network or protocol error.
    pub static ref RPC_ERRORS_TOTAL: Counter = register_counter!(
        Opts::new(
            "rpc_errors_total",
            "Total number of Stellar RPC calls that resulted in an error"
        )
    )
    .expect("failed to register rpc_errors_total counter");
}

/// Render all registered metrics in the Prometheus text exposition format.
pub fn gather_metrics() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    encoder
        .encode_to_string(&metric_families)
        .unwrap_or_default()
}
