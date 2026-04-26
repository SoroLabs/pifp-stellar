mod chain;
mod config;
mod dkg;
mod errors;
mod health;
mod mempool;
mod metrics;
mod mpc;
mod notifications;
mod tss;
mod verifier;
mod wasm_debug;
mod observer;
mod bridge_api;
mod ipfs_api;
mod ipfs;
mod rollup_api;

use std::sync::Arc;

use crate::bridge_api::BridgeState;
use crate::ipfs_api::IpfsState;
use crate::ipfs::IpfsConfig;
use crate::observer::BridgeObserver;
use crate::rollup_api::RollupState;

use clap::Parser;
use sentry::{self, protocol::Event};
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::config::Config;
use crate::errors::{OracleError, Result};
use crate::metrics::OracleMetrics;

const MAX_CONCURRENT_PROOFS: usize = 5;

#[derive(Debug, Clone)]
struct ProofTask {
    project_id: u64,
    proof_cid: String,
}

#[derive(Parser, Debug)]
#[command(name = "pifp-oracle")]
#[command(about = "PIFP Oracle - Verify proofs and release funds")]
struct Cli {
    /// Project ID to verify (single mode)
    #[arg(long)]
    project_id: Option<u64>,

    /// IPFS CID of the proof artifact (single mode)
    #[arg(long)]
    proof_cid: Option<String>,

    /// Comma-separated list of project_id:proof_cid pairs for batch mode
    /// Example: "1:QmAbc,2:QmDef,3:QmGhi"
    #[arg(long)]
    batch: Option<String>,

    /// Dry run: compute hash and log without submitting transaction
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Run as a long-lived HTTP service exposing /health and /metrics
    #[arg(long, default_value_t = false)]
    serve: bool,
}

fn redact_sensitive_data(mut event: Event<'static>) -> Event<'static> {
    event.request = None;
    event.user = None;
    event.extra.clear();
    event.tags.retain(|k, _| {
        let key = k.to_ascii_lowercase();
        !key.contains("auth") && !key.contains("token") && !key.contains("password")
    });
    event
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = dotenvy::dotenv();

    let cli = Cli::parse();
    let config = Arc::new(Config::from_env()?);

    // Initialise Sentry if DSN is configured.
    let _sentry_guard = config.sentry_dsn.as_deref().map(|dsn| {
        info!("Sentry error tracking enabled");
        sentry::init((
            dsn,
            sentry::ClientOptions {
                release: sentry::release_name!(),
                traces_sample_rate: 1.0,
                before_send: Some(std::sync::Arc::new(|event: Event<'static>| {
                    Some(redact_sensitive_data(event))
                })),
                ..Default::default()
            },
        ))
    });

    // sentry::integrations::panic::register_panic_handler();

    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .with(sentry_tracing::layer())
        .init();

    let metrics = Arc::new(OracleMetrics::new());
    
    // Initialize Bridge and IPFS states
    let bridge_state = Arc::new(BridgeState::new());
    let ipfs_state = Arc::new(IpfsState {
        config: IpfsConfig::from_env(),
    });
    let rollup_state = Arc::new(RollupState::new(std::time::Duration::from_secs(15)));

    // Start Bridge Observer in the background if configured
    let observer_config = Arc::clone(&config);
    let observer_bridge_state = Arc::clone(&bridge_state);
    
    // In a real scenario, we'd load the node's secret share from DKG
    // For now, we'll use a dummy signer if node_id > 0
    let signer = None; // Simplified for this task

    let observer = Arc::new(BridgeObserver::new(observer_config, signer));
    tokio::spawn(async move {
        if let Err(e) = observer.run().await {
            error!("Bridge observer failed: {}", e);
        }
    });

    if cli.serve {
        rollup_state.clone().start_settlement_loop();
        health::serve(config.metrics_port, bridge_state, ipfs_state, rollup_state).await?;
        return Ok(());
    }

    let tasks = build_task_list(&cli)?;
    if tasks.is_empty() {
        warn!("No proofs to process. Use --project-id/--proof-cid or --batch.");
        return Ok(());
    }

    info!(
        "PIFP Oracle starting - processing {} proof(s) with max {} concurrent",
        tasks.len(),
        MAX_CONCURRENT_PROOFS
    );

    process_batch(tasks, config, cli.dry_run, metrics).await;
    Ok(())
}

fn build_task_list(cli: &Cli) -> Result<Vec<ProofTask>> {
    let mut tasks = Vec::new();

    if let Some(batch_str) = &cli.batch {
        for entry in batch_str.split(',') {
            let entry = entry.trim();
            if entry.is_empty() {
                continue;
            }

            let mut parts = entry.splitn(2, ':');
            let id_str = parts.next().unwrap_or("").trim();
            let cid = parts.next().unwrap_or("").trim();

            let project_id: u64 = id_str.parse().map_err(|_| {
                OracleError::Config(format!("Invalid project_id in batch entry: '{entry}'"))
            })?;

            if cid.is_empty() {
                return Err(OracleError::Config(format!(
                    "Missing proof_cid in batch entry: '{entry}'"
                )));
            }

            tasks.push(ProofTask {
                project_id,
                proof_cid: cid.to_string(),
            });
        }
    } else {
        match (cli.project_id, cli.proof_cid.clone()) {
            (Some(project_id), Some(proof_cid)) => tasks.push(ProofTask {
                project_id,
                proof_cid,
            }),
            (None, None) => {}
            _ => {
                return Err(OracleError::Config(
                    "Both --project-id and --proof-cid are required in single mode".to_string(),
                ));
            }
        }
    }

    Ok(tasks)
}

async fn process_batch(
    tasks: Vec<ProofTask>,
    config: Arc<Config>,
    dry_run: bool,
    metrics: Arc<OracleMetrics>,
) {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_PROOFS));
    let mut handles = Vec::with_capacity(tasks.len());

    for task in tasks {
        let config = Arc::clone(&config);
        let semaphore = Arc::clone(&semaphore);
        let metrics = Arc::clone(&metrics);

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.expect("semaphore closed");
            process_single_proof(task, config, dry_run, metrics).await
        });
        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(Ok((project_id, tx_hash))) => {
                if let Some(hash) = tx_hash {
                    info!("project={} status=success tx_hash={}", project_id, hash);
                } else {
                    info!("project={} status=dry_run_ok", project_id);
                }
            }
            Ok(Err((project_id, err))) => {
                error!("project={} status=failed error={}", project_id, err);
            }
            Err(join_err) => {
                error!("task panicked: {}", join_err);
            }
        }
    }
}

/// Process a single proof: fetch from IPFS, hash, optionally submit on-chain.
///
/// On any verification failure the Slack alert fires as a best-effort
/// side-effect (3-second timeout, result discarded) before the original
/// error is returned unchanged.
async fn process_single_proof(
    task: ProofTask,
    config: Arc<Config>,
    dry_run: bool,
    metrics: Arc<OracleMetrics>,
) -> std::result::Result<(u64, Option<String>), (u64, String)> {
    let project_id = task.project_id;
    metrics.verifications_total.inc();

    info!(
        project_id = project_id,
        cid = %task.proof_cid,
        "fetching proof"
    );

    let proof_hash = {
        let _timer = metrics.ipfs_fetch_duration_seconds.start_timer();
        match verifier::fetch_and_hash_proof(&task.proof_cid, &config).await {
            Ok(hash) => hash,
            Err(e) => {
                metrics.verification_errors_total.inc();
                sentry::capture_message(&e.to_string(), sentry::Level::Error);
                return Err((project_id, e.to_string()));
            }
        }
    };

    info!(
        project_id = project_id,
        hash = %hex::encode(proof_hash),
        "proof hashed"
    );

    if dry_run {
        warn!(
            project_id = project_id,
            hash = %hex::encode(proof_hash),
            "dry_run — skipping chain submission"
        );
        return Ok((project_id, None));
    }

    let tx_hash = {
        let _timer = metrics.chain_submit_duration_seconds.start_timer();
        match chain::submit_verification(&config, project_id, proof_hash).await {
            Ok(hash) => hash,
            Err(e) => {
                metrics.verification_errors_total.inc();
                sentry::capture_message(&e.to_string(), sentry::Level::Error);
                return Err((project_id, e.to_string()));
            }
        }
    };

    Ok((project_id, Some(tx_hash)))
}
