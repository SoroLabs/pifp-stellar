//! Configuration management for the Oracle service.
//!
//! Loads all required settings from environment variables.

use crate::errors::{OracleError, Result};

#[derive(Debug, Clone)]
pub struct Config {
    /// Soroban RPC endpoint (e.g., https://soroban-testnet.stellar.org)
    pub rpc_url: String,

    /// Horizon API endpoint for transaction submission
    #[allow(dead_code)]
    pub horizon_url: String,

    /// PIFP contract address (Strkey format: C...)
    pub contract_id: String,

    /// Oracle's secret key (Strkey format: S...)
    pub oracle_secret_key: String,

    /// IPFS gateway URL for fetching proof artifacts
    pub ipfs_gateway: String,

    /// Network passphrase (e.g., "Test SDF Network ; September 2015")
    #[allow(dead_code)]
    pub network_passphrase: String,

    /// Request timeout in seconds
    pub timeout_secs: u64,

    /// Optional Sentry DSN for error tracking
    pub sentry_dsn: Option<String>,

    /// Metrics/HTTP API port for the oracle service.
    pub metrics_port: u16,

    /// Foreign chain RPC URL (e.g., Ethereum/Polygon)
    pub foreign_rpc_url: Option<String>,

    /// Foreign bridge contract address
    pub foreign_bridge_address: Option<String>,

    /// Node ID for threshold signatures (1-based)
    pub node_id: usize,

    /// Port for metrics and health endpoints
    pub metrics_port: u16,
}

impl Config {
    /// Load configuration from environment variables.
    pub fn from_env() -> Result<Self> {
        Ok(Config {
            rpc_url: env_var("RPC_URL")
                .unwrap_or_else(|_| "https://soroban-testnet.stellar.org".to_string()),

            horizon_url: env_var("HORIZON_URL")
                .unwrap_or_else(|_| "https://horizon-testnet.stellar.org".to_string()),

            contract_id: env_var("CONTRACT_ID")?,

            oracle_secret_key: env_var("ORACLE_SECRET_KEY")?,

            ipfs_gateway: env_var("IPFS_GATEWAY").unwrap_or_else(|_| "https://ipfs.io".to_string()),

            network_passphrase: env_var("NETWORK_PASSPHRASE")
                .unwrap_or_else(|_| "Test SDF Network ; September 2015".to_string()),

            timeout_secs: env_var("TIMEOUT_SECS")
                .unwrap_or_else(|_| "30".to_string())
                .parse()
                .map_err(|_| OracleError::Config("Invalid TIMEOUT_SECS".to_string()))?,

            sentry_dsn: env_var("SENTRY_DSN").ok(),

            metrics_port: env_var("METRICS_PORT")
                .unwrap_or_else(|_| "9090".to_string())
                .parse()
                .map_err(|_| OracleError::Config("Invalid METRICS_PORT".to_string()))?,

            foreign_rpc_url: env_var("FOREIGN_RPC_URL").ok(),

            foreign_bridge_address: env_var("FOREIGN_BRIDGE_ADDRESS").ok(),

            node_id: env_var("NODE_ID")
                .unwrap_or_else(|_| "1".to_string())
                .parse()
                .map_err(|_| OracleError::Config("Invalid NODE_ID".to_string()))?,

            oracle_asset_symbol: env_var("ORACLE_ASSET_SYMBOL")
                .unwrap_or_else(|_| "XLM".to_string()),

            oracle_quote_symbol: env_var("ORACLE_QUOTE_SYMBOL")
                .unwrap_or_else(|_| "USD".to_string()),

            oracle_refresh_secs: env_var("ORACLE_REFRESH_SECS")
                .unwrap_or_else(|_| "15".to_string())
                .parse()
                .map_err(|_| OracleError::Config("Invalid ORACLE_REFRESH_SECS".to_string()))?,

            oracle_max_staleness_secs: env_var("ORACLE_MAX_STALENESS_SECS")
                .unwrap_or_else(|_| "90".to_string())
                .parse()
                .map_err(|_| OracleError::Config("Invalid ORACLE_MAX_STALENESS_SECS".to_string()))?,

            oracle_max_variance_pct: env_var("ORACLE_MAX_VARIANCE_PCT")
                .unwrap_or_else(|_| "5.0".to_string())
                .parse()
                .map_err(|_| OracleError::Config("Invalid ORACLE_MAX_VARIANCE_PCT".to_string()))?,

            oracle_coingecko_url: env_var("ORACLE_COINGECKO_URL").unwrap_or_else(|_| {
                "https://api.coingecko.com/api/v3/simple/price?ids=stellar&vs_currencies=usd"
                    .to_string()
            }),

            oracle_binance_url: env_var("ORACLE_BINANCE_URL").unwrap_or_else(|_| {
                "https://api.binance.com/api/v3/ticker/price?symbol=XLMUSDT".to_string()
            }),

            oracle_kraken_url: env_var("ORACLE_KRAKEN_URL").unwrap_or_else(|_| {
                "https://api.kraken.com/0/public/Ticker?pair=XLMUSD".to_string()
            }),
        })
    }

    /// Validate that all required configuration is present and well-formed.
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        if !self.contract_id.starts_with('C') {
            return Err(OracleError::Config(
                "CONTRACT_ID must be a valid Stellar contract address (starts with 'C')"
                    .to_string(),
            ));
        }
        if !self.oracle_secret_key.starts_with('S') {
            return Err(OracleError::Config(
                "ORACLE_SECRET_KEY must be a valid Stellar secret key (starts with 'S')"
                    .to_string(),
            ));
        }
        if !self.rpc_url.starts_with("http") {
            return Err(OracleError::Config(
                "RPC_URL must be a valid HTTP(S) URL".to_string(),
            ));
        }
        if !self.horizon_url.starts_with("http") {
            return Err(OracleError::Config(
                "HORIZON_URL must be a valid HTTP(S) URL".to_string(),
            ));
        }
        if !self.ipfs_gateway.starts_with("http") {
            return Err(OracleError::Config(
                "IPFS_GATEWAY must be a valid HTTP(S) URL".to_string(),
            ));
        }
        Ok(())
    }
}

fn env_var(key: &str) -> Result<String> {
    std::env::var(key)
        .map_err(|_| OracleError::Config(format!("Missing required environment variable: {key}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_contract_id() {
        let mut config = mock_config();
        config.contract_id = "INVALID".to_string();
        assert!(config.validate().is_err());
        config.contract_id = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM".to_string();
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_validate_secret_key() {
        let mut config = mock_config();
        config.oracle_secret_key = "INVALID".to_string();
        assert!(config.validate().is_err());
        config.oracle_secret_key =
            "SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
        assert!(config.validate().is_ok());
    }

    fn mock_config() -> Config {
        Config {
            rpc_url: "https://soroban-testnet.stellar.org".to_string(),
            horizon_url: "https://horizon-testnet.stellar.org".to_string(),
            contract_id: "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM".to_string(),
            oracle_secret_key: "SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(),
            ipfs_gateway: "https://ipfs.io".to_string(),
            network_passphrase: "Test SDF Network ; September 2015".to_string(),
            timeout_secs: 30,
            sentry_dsn: None,
            metrics_port: 9090,
            foreign_rpc_url: None,
            foreign_bridge_address: None,
            node_id: 1,
            oracle_asset_symbol: "XLM".to_string(),
            oracle_quote_symbol: "USD".to_string(),
            oracle_refresh_secs: 15,
            oracle_max_staleness_secs: 90,
            oracle_max_variance_pct: 5.0,
            oracle_coingecko_url: "https://api.coingecko.com/api/v3/simple/price?ids=stellar&vs_currencies=usd".to_string(),
            oracle_binance_url: "https://api.binance.com/api/v3/ticker/price?symbol=XLMUSDT".to_string(),
            oracle_kraken_url: "https://api.kraken.com/0/public/Ticker?pair=XLMUSD".to_string(),
        }
    }
}

