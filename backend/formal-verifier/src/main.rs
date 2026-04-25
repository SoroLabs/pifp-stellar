use std::fs;

use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{prelude::*, EnvFilter};

use pifp_formal_verifier::{verify_upgrade, PifpInvariantProfile, VerificationConfig};

#[derive(Debug, Parser)]
#[command(name = "pifp-formal-verifier")]
#[command(about = "Bounded symbolic verifier for PIFP contract upgrades")]
struct Cli {
    /// Path to the proposed WASM binary.
    #[arg(long)]
    wasm: String,

    /// Comma-separated list of exports to focus on.
    #[arg(long)]
    focus: Option<String>,

    /// Maximum loop unroll depth.
    #[arg(long, default_value_t = 2)]
    max_loop_unroll: usize,

    /// Fail closed when unsupported opcodes are encountered.
    #[arg(long, default_value_t = true)]
    fail_closed_on_unsupported: bool,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let cli = Cli::parse();
    let wasm = fs::read(&cli.wasm)?;
    let config = VerificationConfig {
        max_loop_unroll: cli.max_loop_unroll,
        focus_exports: cli
            .focus
            .as_deref()
            .map(|items| items.split(',').map(|s| s.trim().to_string()).collect()),
        fail_closed_on_unsupported: cli.fail_closed_on_unsupported,
    };

    let profile = PifpInvariantProfile::default();
    match verify_upgrade(&wasm, &profile, &config) {
        Ok(report) => {
            info!(
                safe = report.safe,
                checked = report.checked_functions.len(),
                "formal verification completed"
            );
            println!("{}", serde_json::to_string_pretty(&report)?);
            Ok(())
        }
        Err(err) => {
            error!("{err}");
            println!("{err}");
            std::process::exit(1);
        }
    }
}
