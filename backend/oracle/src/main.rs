mod chain;
mod config;
mod errors;
mod health;
mod metrics;
mod verifier;

use std::sync::Arc;

use clap::Parser;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::errors::Result;
use crate::metrics::OracleMetrics;

#[derive(Parser, Debug)]
#[command(name = "pifp-oracle")]
#[command(about = "PIFP Oracle - Verify proofs and release funds")]
struct Cli {
    /// Project ID to verify
    #[arg(long, required_unless_present = "serve")]
    project_id: Option<u64>,

    /// IPFS CID of the proof artifact
    #[arg(long, required_unless_present = "serve")]
    proof_cid: Option<String>,

    /// Dry run — compute hash and log without submitting transaction
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Run as a long-lived HTTP service exposing /health and /metrics
    #[arg(long, default_value_t = false)]
    serve: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let _ = dotenvy::dotenv();
    let cli = Cli::parse();
    let config = Config::from_env().map_err(|e| anyhow::anyhow!("{e}"))?;

    // Initialise Sentry if DSN is configured.
    let _sentry_guard = config.sentry_dsn.as_deref().map(|dsn| {
        info!("Sentry error tracking enabled");
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 1.0,
                ..Default::default()
            },
        ))
    });

    let metrics = Arc::new(OracleMetrics::new());

    if cli.serve {
        // Long-lived service mode: spawn health/metrics server and block.
        health::serve(config.metrics_port).await?;
        return Ok(());
    }

    // One-shot verification mode.
    let project_id = cli.project_id.expect("project-id required");
    let proof_cid = cli.proof_cid.expect("proof-cid required");

    info!(
        "PIFP Oracle starting — project_id={}, proof_cid={}",
        project_id, proof_cid
    );

    metrics.verifications_total.inc();

    // Step 1: Fetch proof from IPFS and compute hash.
    let proof_hash = {
        let _timer = metrics.ipfs_fetch_duration_seconds.start_timer();
        match verifier::fetch_and_hash_proof(&proof_cid, &config).await {
            Ok(h) => h,
            Err(e) => {
                metrics.verification_errors_total.inc();
                sentry::capture_message(&e.to_string(), sentry::Level::Error);
                error!("IPFS fetch failed: {e}");
                return Err(anyhow::anyhow!("{e}"));
            }
        }
    };
    info!("Proof hash: {}", hex::encode(proof_hash));

    if cli.dry_run {
        warn!("DRY RUN — transaction will not be submitted");
        return Ok(());
    }

    // Step 2: Submit verify_and_release transaction.
    let tx_hash = {
        let _timer = metrics.chain_submit_duration_seconds.start_timer();
        match chain::submit_verification(&config, project_id, proof_hash).await {
            Ok(h) => h,
            Err(e) => {
                metrics.verification_errors_total.inc();
                sentry::capture_message(&e.to_string(), sentry::Level::Error);
                error!("Chain submission failed: {e}");
                return Err(anyhow::anyhow!("{e}"));
            }
        }
    };

    info!("Verification submitted — tx={}", tx_hash);
    Ok(())
}
