use std::fs;

use clap::Parser;
use tokio::sync::mpsc;
use tracing::{info, warn};
use tracing_subscriber::{prelude::*, EnvFilter};

use pifp_execution_engine::{
    build_execution_plan, feed::stream_pool_snapshots, BellmanFordFinder, FeePolicy, PoolSnapshot,
};

#[derive(Debug, Parser)]
#[command(name = "pifp-execution-engine")]
#[command(about = "Low-latency arbitrage route discovery for Stellar AMM pools")]
struct Cli {
    /// Optional path to a JSON file containing one snapshot or a list of snapshots.
    #[arg(long)]
    snapshot_file: Option<String>,

    /// Optional websocket feed URL that streams pool snapshots.
    #[arg(long)]
    ws_url: Option<String>,

    /// Trade size in stroops used to price the opportunity.
    #[arg(long, default_value_t = 1_000_000)]
    notional_stroops: u64,

    /// Minimum profit capture in basis points.
    #[arg(long, default_value_t = 1_500)]
    profit_capture_bps: u16,

    /// Base Stellar fee in stroops.
    #[arg(long, default_value_t = 100)]
    base_fee_stroops: u32,

    /// Maximum fee budget in stroops.
    #[arg(long, default_value_t = 50_000)]
    max_fee_stroops: u32,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(tracing_subscriber::fmt::layer())
        .init();

    let Cli {
        snapshot_file,
        ws_url,
        notional_stroops,
        profit_capture_bps,
        base_fee_stroops,
        max_fee_stroops,
    } = Cli::parse();

    let finder = BellmanFordFinder::default();

    if let Some(path) = snapshot_file {
        let snapshots = load_snapshots(&path)?;
        run_once(
            &finder,
            &snapshots,
            notional_stroops,
            base_fee_stroops,
            max_fee_stroops,
            profit_capture_bps,
        );
        return Ok(());
    }

    if let Some(ws_url) = ws_url {
        let (sender, mut receiver) = mpsc::channel::<PoolSnapshot>(256);
        let forwarder = tokio::spawn(async move {
            if let Err(err) = stream_pool_snapshots(&ws_url, sender).await {
                warn!("websocket feed exited: {err}");
            }
        });

        let mut snapshots = Vec::new();
        while let Some(snapshot) = receiver.recv().await {
            snapshots.push(snapshot);
            run_once(
                &finder,
                &snapshots,
                notional_stroops,
                base_fee_stroops,
                max_fee_stroops,
                profit_capture_bps,
            );
        }

        let _ = forwarder.await;
        return Ok(());
    }

    warn!("no input source provided; use --snapshot-file or --ws-url");
    Ok(())
}

fn load_snapshots(path: &str) -> anyhow::Result<Vec<PoolSnapshot>> {
    let data = fs::read_to_string(path)?;
    if let Ok(batch) = serde_json::from_str::<Vec<PoolSnapshot>>(&data) {
        return Ok(batch);
    }

    let single = serde_json::from_str::<PoolSnapshot>(&data)?;
    Ok(vec![single])
}

fn run_once(
    finder: &BellmanFordFinder,
    snapshots: &[PoolSnapshot],
    notional_stroops: u64,
    base_fee_stroops: u32,
    max_fee_stroops: u32,
    profit_capture_bps: u16,
) {
    let mut policy = FeePolicy::default();
    policy.base_fee_stroops = base_fee_stroops;
    policy.max_fee_stroops = max_fee_stroops;
    policy.profit_capture_bps = profit_capture_bps;

    match finder.find_best_opportunity(snapshots) {
        Some(opportunity) => {
            if let Some(plan) = build_execution_plan(&opportunity, notional_stroops, &policy) {
                info!(
                    gross_profit_bps = plan.opportunity.gross_profit_bps,
                    fee_stroops = plan.fee_bump.total_fee_stroops,
                    route_len = plan.opportunity.route.len(),
                    "arbitrage plan ready"
                );
                println!("{}", serde_json::to_string_pretty(&plan).unwrap());
            } else {
                warn!("opportunity found but fee budget is not attractive enough");
            }
        }
        None => {
            info!("no profitable opportunity detected");
        }
    }
}
