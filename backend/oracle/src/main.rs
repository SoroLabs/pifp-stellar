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

use clap::Parser;
use tracing::{info, warn};
use tracing_subscriber::EnvFilter;

use crate::config::Config;
use crate::errors::Result;

#[derive(Parser, Debug)]
#[command(name = "pifp-oracle")]
#[command(about = "PIFP Oracle - Verify proofs and release funds", long_about = None)]
struct Cli {
    /// Project ID to verify
    #[arg(long)]
    project_id: u64,

    /// IPFS CID of the proof artifact
    #[arg(long)]
    proof_cid: String,

    /// Dry run mode - compute hash and log without submitting transaction
    #[arg(long, default_value_t = false)]
    dry_run: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    // Load .env file if present
    let _ = dotenvy::dotenv();

    // Parse CLI arguments
    let cli = Cli::parse();

    info!(
        "PIFP Oracle starting - Project ID: {}, Proof CID: {}",
        cli.project_id, cli.proof_cid
    );

    // Load configuration from environment
    let config = Config::from_env()?;

    // Step 1: Fetch proof from IPFS and compute hash
    info!("Fetching proof from IPFS: {}", cli.proof_cid);
    let proof_hash = verifier::fetch_and_hash_proof(&cli.proof_cid, &config).await?;
    info!("Computed proof hash: {}", hex::encode(proof_hash));

    if cli.dry_run {
        warn!("DRY RUN MODE - Transaction will not be submitted");
        info!(
            "Would submit verify_and_release for project {} with hash {}",
            cli.project_id,
            hex::encode(proof_hash)
        );
        return Ok(());
    }

    // Step 2: Submit verify_and_release transaction
    info!("Submitting verify_and_release transaction to contract");
    let tx_hash = chain::submit_verification(&config, cli.project_id, proof_hash).await?;

    info!("✓ Verification transaction submitted successfully!");
    info!("Transaction hash: {}", tx_hash);
    info!("Project {} funds released", cli.project_id);

    Ok(())
}
