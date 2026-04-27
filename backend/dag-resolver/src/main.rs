use std::fs;
use std::io::{self, Read};
use std::path::PathBuf;

use clap::Parser;
use tracing::{error, info};
use tracing_subscriber::{prelude::*, EnvFilter};

use pifp_dag_resolver::{resolve_intents, IntentDocument};

#[derive(Debug, Parser)]
#[command(name = "pifp-dag-resolver")]
#[command(about = "Dependency-aware batching for Soroban transaction intents")]
struct Cli {
    /// Path to a JSON file containing intents, or '-' for stdin.
    #[arg(long, short = 'i', default_value = "-")]
    intents: String,

    /// Emit compact JSON instead of pretty-printed output.
    #[arg(long, default_value_t = false)]
    compact: bool,
}

fn main() {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    if let Err(err) = run() {
        error!("{err}");
        println!("{err}");
        std::process::exit(1);
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let bytes = if cli.intents == "-" {
        let mut input = Vec::new();
        io::stdin().read_to_end(&mut input)?;
        input
    } else {
        fs::read(PathBuf::from(&cli.intents))?
    };

    let document: IntentDocument = serde_json::from_slice(&bytes)?;
    let report = resolve_intents(document.into_intents())?;
    let rendered = if cli.compact {
        serde_json::to_string(&report)?
    } else {
        serde_json::to_string_pretty(&report)?
    };

    info!(
        total_intents = report.total_intents,
        total_edges = report.total_edges,
        max_parallel_width = report.max_parallel_width,
        "dag resolution completed"
    );
    println!("{rendered}");
    Ok(())
}
