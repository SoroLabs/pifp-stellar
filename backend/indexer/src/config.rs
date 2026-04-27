//! Application configuration loaded from environment variables.

use crate::errors::{ IndexerError, Result };

#[derive(Debug, Clone)]
pub struct Config {
    /// Primary Soroban/Horizon RPC endpoint (e.g. https://soroban-testnet.stellar.org)
    pub rpc_url: String,
    /// Ordered list of fallback RPC URLs tried when the primary is unhealthy.
    /// Comma-separated via `RPC_FALLBACK_URLS`.
    pub rpc_fallback_urls: Vec<String>,
    /// Seconds a failed provider stays in cool-down before being re-enabled.
    pub rpc_cooldown_secs: u64,
    /// PIFP contract addresses (Strkey format). Supports multi-deployment indexing.
    pub contract_ids: Vec<String>,
    /// Path to the SQLite database file
    pub database_url: String,
    /// Port for the REST API server
    pub api_port: u16,
    /// Port for the Prometheus /metrics endpoint
    /// Port for the Prometheus metrics server
    pub metrics_port: u16,
    /// Port for the WebSocket server
    pub ws_port: u16,
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
    /// Optional Sentry DSN for error tracking
    pub sentry_dsn: Option<String>,
    /// Optional API rate limit (requests per minute)
    pub api_rate_limit: Option<u32>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let contract_ids = parse_contract_ids()?;
        Ok(Config {
            rpc_url: env_var("RPC_URL").unwrap_or_else(|_|
                "https://soroban-testnet.stellar.org".to_string()
            ),
            rpc_fallback_urls: std::env
                ::var("RPC_FALLBACK_URLS")
                .unwrap_or_default()
                .split(',')
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(ToString::to_string)
                .collect(),
            rpc_cooldown_secs: env_var("RPC_COOLDOWN_SECS")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid RPC_COOLDOWN_SECS".to_string()))?,
            contract_ids,
            database_url: env_var("DATABASE_URL").unwrap_or_else(|_|
                "sqlite:./pifp_events.db".to_string()
            ),
            api_port: env_var("API_PORT")
                .unwrap_or_else(|_| "3001".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid API_PORT".to_string()))?,
            metrics_port: env_var("METRICS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid METRICS_PORT".to_string()))?,
            ws_port: env_var("WS_PORT")
                .unwrap_or_else(|_| "3002".to_string())
                .parse()
                .map_err(|_| IndexerError::Config("Invalid WS_PORT".to_string()))?,
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
            backfill_start_ledger: std::env
                ::var("BACKFILL_START_LEDGER")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(|v| {
                    v.parse::<u32>().map_err(|_| {
                        IndexerError::Config("Invalid BACKFILL_START_LEDGER".to_string())
                    })
                })
                .transpose()?,
            backfill_cursor: std::env
                ::var("BACKFILL_CURSOR")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            redis_url: std::env
                ::var("REDIS_URL")
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
            sentry_dsn: env_var("SENTRY_DSN").ok(),
            api_rate_limit: std::env
                ::var("API_RATE_LIMIT")
                .ok()
                .and_then(|v| v.parse().ok()),
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
            "Set CONTRACT_ID (single) or CONTRACT_IDS (comma-separated)".to_string()
        )
    })?;
    Ok(vec![single])
}
