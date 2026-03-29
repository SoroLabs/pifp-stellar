use std::time::Duration;

use reqwest::{
    multipart::{Form, Part},
    Client,
};
use serde::Deserialize;
use tracing::{info, warn};

use crate::errors::{OracleError, Result};

const PINATA_PIN_URL: &str = "https://api.pinata.cloud/pinning/pinFileToIPFS";
const WEB3_STORAGE_URL: &str = "https://api.web3.storage/upload";

const MAX_RETRIES: u32 = 3;
const BASE_BACKOFF_MS: u64 = 500;
const REQUEST_TIMEOUT_SECS: u64 = 60;

#[derive(Debug, Clone)]
pub struct IpfsConfig {
    pub pinata_api_key: Option<String>,
    pub pinata_api_secret: Option<String>,
    pub web3_storage_token: Option<String>,
}

impl IpfsConfig {
    pub fn from_env() -> Self {
        IpfsConfig {
            pinata_api_key: std::env::var("PINATA_API_KEY").ok(),
            pinata_api_secret: std::env::var("PINATA_API_SECRET").ok(),
            web3_storage_token: std::env::var("WEB3_STORAGE_TOKEN").ok(),
        }
    }
}

#[derive(Deserialize)]
struct PinataResponse {
    #[serde(rename = "IpfsHash")]
    ipfs_hash: String,
}

#[derive(Deserialize)]
struct Web3StorageResponse {
    cid: String,
}

pub async fn pin_file(data: Vec<u8>, config: &IpfsConfig) -> Result<String> {
    if config.pinata_api_key.is_some() && config.pinata_api_secret.is_some() {
        info!("Attempting to pin via Pinata");
        match pin_with_retry(data.clone(), config, pin_via_pinata).await {
            Ok(cid) => {
                info!(cid = %cid, "Pinned via Pinata");
                return Ok(cid);
            }
            Err(e) => {
                warn!(error = %e, "Pinata failed, falling back to Web3.Storage");
            }
        }
    }

    if config.web3_storage_token.is_some() {
        info!("Attempting to pin via Web3.Storage");
        match pin_with_retry(data, config, pin_via_web3_storage).await {
            Ok(cid) => {
                info!(cid = %cid, "Pinned via Web3.Storage");
                return Ok(cid);
            }
            Err(e) => {
                warn!(error = %e, "Web3.Storage failed");
                return Err(e);
            }
        }
    }

    Err(OracleError::Config(
        "No IPFS pinning credentials configured. Set PINATA_API_KEY/PINATA_API_SECRET or WEB3_STORAGE_TOKEN".to_string(),
    ))
}

async fn pin_with_retry<F, Fut>(
    data: Vec<u8>,
    config: &IpfsConfig,
    pin_fn: F,
) -> Result<String>
where
    F: Fn(Vec<u8>, &IpfsConfig, Client) -> Fut,
    Fut: std::future::Future<Output = Result<String>>,
{
    let mut last_err = OracleError::Network("No attempts made".to_string());

    for attempt in 0..MAX_RETRIES {
        if attempt > 0 {
            let backoff = BASE_BACKOFF_MS * (1 << (attempt - 1));
            warn!(attempt, backoff_ms = backoff, "Retrying IPFS pin after backoff");
            tokio::time::sleep(Duration::from_millis(backoff)).await;
        }

        let client = build_client()?;

        match pin_fn(data.clone(), config, client).await {
            Ok(cid) => return Ok(cid),
            Err(e) => {
                warn!(attempt, error = %e, "IPFS pin attempt failed");
                last_err = e;
            }
        }
    }

    Err(last_err)
}

async fn pin_via_pinata(data: Vec<u8>, config: &IpfsConfig, client: Client) -> Result<String> {
    let api_key = config
        .pinata_api_key
        .as_deref()
        .ok_or_else(|| OracleError::Config("PINATA_API_KEY not set".to_string()))?;
    let api_secret = config
        .pinata_api_secret
        .as_deref()
        .ok_or_else(|| OracleError::Config("PINATA_API_SECRET not set".to_string()))?;

    let part = Part::bytes(data)
        .file_name("proof.bin")
        .mime_str("application/octet-stream")
        .map_err(|e| OracleError::Network(format!("Failed to build multipart part: {e}")))?;

    let form = Form::new().part("file", part);

    let response = client
        .post(PINATA_PIN_URL)
        .header("pinata_api_key", api_key)
        .header("pinata_secret_api_key", api_secret)
        .multipart(form)
        .send()
        .await
        .map_err(|e| OracleError::Network(format!("Pinata request failed: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OracleError::Network(format!(
            "Pinata returned {status}: {body}"
        )));
    }

    let pinata_resp: PinataResponse = response
        .json()
        .await
        .map_err(|e| OracleError::Network(format!("Failed to parse Pinata response: {e}")))?;

    Ok(pinata_resp.ipfs_hash)
}

async fn pin_via_web3_storage(
    data: Vec<u8>,
    config: &IpfsConfig,
    client: Client,
) -> Result<String> {
    let token = config
        .web3_storage_token
        .as_deref()
        .ok_or_else(|| OracleError::Config("WEB3_STORAGE_TOKEN not set".to_string()))?;

    let response = client
        .post(WEB3_STORAGE_URL)
        .bearer_auth(token)
        .header("Content-Type", "application/octet-stream")
        .body(data)
        .send()
        .await
        .map_err(|e| OracleError::Network(format!("Web3.Storage request failed: {e}")))?;

    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(OracleError::Network(format!(
            "Web3.Storage returned {status}: {body}"
        )));
    }

    let web3_resp: Web3StorageResponse = response
        .json()
        .await
        .map_err(|e| OracleError::Network(format!("Failed to parse Web3.Storage response: {e}")))?;

    Ok(web3_resp.cid)
}

fn build_client() -> Result<Client> {
    Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|e| OracleError::Network(format!("Failed to build HTTP client: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipfs_config_missing_credentials() {
        let config = IpfsConfig {
            pinata_api_key: None,
            pinata_api_secret: None,
            web3_storage_token: None,
        };
        assert!(config.pinata_api_key.is_none());
        assert!(config.web3_storage_token.is_none());
    }

    #[tokio::test]
    async fn test_pin_file_no_credentials_returns_error() {
        let config = IpfsConfig {
            pinata_api_key: None,
            pinata_api_secret: None,
            web3_storage_token: None,
        };
        let result = pin_file(b"test data".to_vec(), &config).await;
        assert!(result.is_err());
        match result {
            Err(OracleError::Config(msg)) => assert!(msg.contains("No IPFS pinning credentials")),
            _ => panic!("Expected Config error"),
        }
    }

    #[test]
    fn test_build_client_succeeds() {
        assert!(build_client().is_ok());
    }
}
