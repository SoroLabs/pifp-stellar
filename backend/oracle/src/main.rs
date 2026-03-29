mod chain;
mod config;
mod errors;
mod health;
mod metrics;
mod verifier;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::errors::Result;
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
    /// Project ID to verify
    #[arg(long, required_unless_present = "serve")]
    project_id: Option<u64>,

    /// IPFS CID of the proof artifact
    #[arg(long, required_unless_present = "serve")]
    proof_cid: Option<String>,
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

    /// Dry run — compute hash and log without submitting transaction
    #[arg(long, default_value_t = false)]
    dry_run: bool,

    /// Run as a long-lived HTTP service exposing /health and /metrics
    #[arg(long, default_value_t = false)]
    serve: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
async fn main() -> Result<()> {
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

    let config = Arc::new(Config::from_env()?);

    let tasks = build_task_list(&cli)?;

    if tasks.is_empty() {
        warn!("No proofs to process. Use --project-id/--proof-cid or --batch.");
        return Ok(());
    }

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
        "PIFP Oracle starting - processing {} proof(s) with max {} concurrent",
        tasks.len(),
        MAX_CONCURRENT_PROOFS
    );

    process_batch(tasks, config, cli.dry_run).await;

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
                crate::errors::OracleError::Config(format!(
                    "Invalid project_id in batch entry: '{entry}'"
                ))
            })?;

            if cid.is_empty() {
                return Err(crate::errors::OracleError::Config(format!(
                    "Missing proof_cid in batch entry: '{entry}'"
                )));
            }

            tasks.push(ProofTask {
                project_id,
                proof_cid: cid.to_string(),
            });
        }
    } else if let (Some(project_id), Some(proof_cid)) = (cli.project_id, cli.proof_cid.clone()) {
        tasks.push(ProofTask {
            project_id,
            proof_cid,
        });
    }

    Ok(tasks)
}

async fn process_batch(tasks: Vec<ProofTask>, config: Arc<Config>, dry_run: bool) {
    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_PROOFS));
    let mut handles = Vec::with_capacity(tasks.len());

    for task in tasks {
        let config = Arc::clone(&config);
        let semaphore = Arc::clone(&semaphore);

        let handle = tokio::spawn(async move {
            let _permit = semaphore.acquire().await.expect("semaphore closed");
            process_single_proof(task, config, dry_run).await
        });

        handles.push(handle);
    }

    for handle in handles {
        match handle.await {
            Ok(Ok((project_id, tx_hash))) => {
                if let Some(hash) = tx_hash {
                    info!(
                        "project={} status=success tx_hash={}",
                        project_id, hash
                    );
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

async fn process_single_proof(
    task: ProofTask,
    config: Arc<Config>,
    dry_run: bool,
) -> std::result::Result<(u64, Option<String>), (u64, String)> {
    let project_id = task.project_id;

    info!(
        "project={} cid={} status=fetching",
        project_id, task.proof_cid
    );

    let proof_hash = verifier::fetch_and_hash_proof(&task.proof_cid, &config)
        .await
        .map_err(|e| (project_id, e.to_string()))?;

    info!(
        "project={} hash={} status=hashed",
        project_id,
        hex::encode(proof_hash)
    );

    if dry_run {
        warn!(
            "project={} status=dry_run would submit verify_and_release with hash={}",
            project_id,
            hex::encode(proof_hash)
        );
        return Ok((project_id, None));
    }

    let tx_hash = chain::submit_verification(&config, project_id, proof_hash)
        .await
        .map_err(|e| (project_id, e.to_string()))?;

    Ok((project_id, Some(tx_hash)))
}
