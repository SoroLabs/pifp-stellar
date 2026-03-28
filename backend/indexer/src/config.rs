//! Application configuration loaded from environment variables.

use crate::errors::{IndexerError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    /// Soroban/Horizon RPC endpoint (e.g. https://soroban-testnet.stellar.org)
    pub rpc_url: String,
    /// PIFP contract addresses (Strkey format). Supports multi-deployment indexing.
    pub contract_ids: Vec<String>,
    /// Path to the SQLite database file
    pub database_url: String,
    /// Port for the REST API server
    pub api_port: u16,
    /// How often (in seconds) to poll the RPC for new events
    pub poll_interval_secs: u64,
    /// Maximum number of events to fetch per RPC request
    pub events_per_page: u32,
    /// Ledger to start from if no cursor is saved
    pub start_ledger: u32,
    /// Optional explicit backfill starting ledger (overrides persisted cursor start).
    pub backfill_start_ledger: Option<u32>,
    /// Optional explicit backfill cursor (if provided, used as initial RPC cursor).
    pub backfill_cursor: Option<String>,
    /// Optional Redis endpoint used for API response caching
    pub redis_url: Option<String>,
    /// TTL for top projects cache entries (seconds)
    pub cache_ttl_top_projects_secs: u64,
    /// TTL for active projects count cache entries (seconds)
    pub cache_ttl_active_projects_count_secs: u64,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let contract_ids = parse_contract_ids()?;
        Ok(Config {
            rpc_url: env_var("RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),
            contract_ids,
            database_url: env_var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:./pifp_events.db".to_string()),
            api_port: env_var("API_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid API_PORT".to_string()))?,
            poll_interval_secs: env_var("POLL_INTERVAL_SECS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid POLL_INTERVAL_SECS".to_string()))?,
            events_per_page: env_var("EVENTS_PER_PAGE")
                .unwrap_or_else(|_| "100".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid EVENTS_PER_PAGE".to_string()))?,
            start_ledger: env_var("START_LEDGER")
                .unwrap_or_else(|_| "0".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid START_LEDGER".to_string()))?,
            backfill_start_ledger: std::env::var("BACKFILL_START_LEDGER")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|v| {
                    v.parse::<u32>().map_err(|_| {
                        IndexerError::Config("Invalid BACKFILL_START_LEDGER".to_string())
                    })
                })
                .transpose()?,
            backfill_cursor: std::env::var("BACKFILL_CURSOR")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            redis_url: std::env::var("REDIS_URL")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            cache_ttl_top_projects_secs: env_var("CACHE_TTL_TOP_PROJECTS_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .map_err(|_| {
                    IndexerError::Config("Invalid CACHE_TTL_TOP_PROJECTS_SECS".to_string())
                })?,
            cache_ttl_active_projects_count_secs: env_var("CACHE_TTL_ACTIVE_PROJECTS_COUNT_SECS")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .map_err(|_| {
                    IndexerError::Config("Invalid CACHE_TTL_ACTIVE_PROJECTS_COUNT_SECS".to_string())
                })?,
        })
    }
}

fn env_var(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| IndexerError::Config(format!("Missing env var: {key}")))
}

fn parse_contract_ids() -> Result<Vec<String>> {
    if let Ok(ids) = std::env::var("CONTRACT_IDS") {
        let parsed: Vec<String> = ids
            .split(',')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string)
            .collect();
        if !parsed.is_empty() {
            return Ok(parsed);
        }
    }

    let single = env_var("CONTRACT_ID").map_err(|_| {
        IndexerError::Config(
            "Set CONTRACT_ID (single) or CONTRACT_IDS (comma-separated)".to_string(),
        )
    })?;
    Ok(vec![single])
}
