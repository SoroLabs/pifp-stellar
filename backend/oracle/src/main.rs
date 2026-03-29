//! PIFP Oracle Service
//!
//! Standalone service that:
//! 1. Fetches proof artifacts from IPFS
//! 2. Computes SHA-256 hash
//! 3. Submits verify_and_release transaction to the Soroban contract

mod chain;
mod config;
mod errors;
mod verifier;

use std::sync::Arc;

use clap::Parser;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::errors::Result;

const MAX_CONCURRENT_PROOFS: usize = 5;

#[derive(Debug, Clone)]
struct ProofTask {
    project_id: u64,
    proof_cid: String,
}

#[derive(Parser, Debug)]
#[command(name = "pifp-oracle")]
#[command(about = "PIFP Oracle - Verify proofs and release funds", long_about = None)]
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

    /// Dry run mode - compute hash and log without submitting transaction
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    let _ = dotenvy::dotenv();

    let cli = Cli::parse();

    let config = Arc::new(Config::from_env()?);

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
