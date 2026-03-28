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
}

impl Config {
    /// Load configuration from environment variables.
    ///
    /// Required variables:
    /// - `RPC_URL`: Soroban RPC endpoint
    /// - `CONTRACT_ID`: PIFP contract address
    /// - `ORACLE_SECRET_KEY`: Oracle's signing key
    ///
    /// Optional variables (with defaults):
    /// - `HORIZON_URL`: Horizon API endpoint (defaults to testnet)
    /// - `IPFS_GATEWAY`: IPFS gateway (defaults to ipfs.io)
    /// - `NETWORK_PASSPHRASE`: Network passphrase (defaults to testnet)
    /// - `TIMEOUT_SECS`: Request timeout (defaults to 30)
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
        })
    }

    /// Validate that all required configuration is present and well-formed.
    #[allow(dead_code)]
    pub fn validate(&self) -> Result<()> {
        // Validate contract ID format (should start with 'C')
        if !self.contract_id.starts_with('C') {
            return Err(OracleError::Config(
                "CONTRACT_ID must be a valid Stellar contract address (starts with 'C')"
                    .to_string(),
            ));
        }

        // Validate secret key format (should start with 'S')
        if !self.oracle_secret_key.starts_with('S') {
            return Err(OracleError::Config(
                "ORACLE_SECRET_KEY must be a valid Stellar secret key (starts with 'S')"
                    .to_string(),
            ));
        }

        // Validate URLs
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
        }
    }
}
