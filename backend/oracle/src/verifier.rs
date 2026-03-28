//! Proof verification logic.
//!
//! Fetches proof artifacts from IPFS and computes their SHA-256 hash.

use reqwest::Client;
use sha2::{Digest, Sha256};
use tracing::{debug, info};

use crate::config::Config;
use crate::errors::{OracleError, Result};

/// Fetch proof artifact from IPFS and compute its SHA-256 hash.
///
/// # Arguments
/// * `cid` - IPFS Content Identifier (e.g., "QmXxx...")
/// * `config` - Oracle configuration containing IPFS gateway URL
///
/// # Returns
/// 32-byte SHA-256 hash of the proof artifact
///
/// # Errors
/// Returns error if:
/// - IPFS fetch fails (network error, 404, timeout)
/// - Response body is empty
/// - Response exceeds reasonable size limit (100MB)
pub async fn fetch_and_hash_proof(cid: &str, config: &Config) -> Result<[u8; 32]> {
    // Construct IPFS gateway URL
    let url = format!("{}/ipfs/{}", config.ipfs_gateway, cid);
    info!("Fetching proof from: {}", url);

    // Create HTTP client with timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| OracleError::Network(format!("Failed to create HTTP client: {e}")))?;

    // Fetch proof artifact
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| OracleError::Network(format!("IPFS fetch failed: {e}")))?;

    // Check response status
    if !response.status().is_success() {
        return Err(OracleError::ProofNotFound(format!(
            "IPFS returned status {}: CID {} not found or inaccessible",
            response.status(),
            cid
        )));
    }

    // Get content length if available
    let content_length = response.content_length();
    if let Some(len) = content_length {
        debug!("Proof artifact size: {} bytes", len);

        // Sanity check: reject files larger than 100MB
        const MAX_SIZE: u64 = 100 * 1024 * 1024;
        if len > MAX_SIZE {
            return Err(OracleError::Verification(format!(
                "Proof artifact too large: {len} bytes (max: {MAX_SIZE} bytes)"
            )));
        }
    }

    // Read response body
    let bytes = response
        .bytes()
        .await
        .map_err(|e| OracleError::Network(format!("Failed to read response body: {e}")))?;

    if bytes.is_empty() {
        return Err(OracleError::Verification(
            "Proof artifact is empty".to_string(),
        ));
    }

    info!("Downloaded {} bytes from IPFS", bytes.len());

    // Compute SHA-256 hash
    let hash = compute_sha256(&bytes);
    debug!("SHA-256 hash: {}", hex::encode(hash));

    Ok(hash)
}

/// Compute SHA-256 hash of the given data.
///
/// # Arguments
/// * `data` - Raw bytes to hash
///
/// # Returns
/// 32-byte SHA-256 digest
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compute_sha256_empty() {
        let data = b"";
        let hash = compute_sha256(data);

        // SHA-256 of empty string
        let expected =
            hex::decode("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
                .unwrap();

        assert_eq!(hash.as_slice(), expected.as_slice());
    }

    #[test]
    fn test_compute_sha256_hello_world() {
        let data = b"hello world";
        let hash = compute_sha256(data);

        // SHA-256 of "hello world"
        let expected =
            hex::decode("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")
                .unwrap();

        assert_eq!(hash.as_slice(), expected.as_slice());
    }

    #[test]
    fn test_compute_sha256_deterministic() {
        let data = b"test data";
        let hash1 = compute_sha256(data);
        let hash2 = compute_sha256(data);

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_fetch_invalid_cid() {
        let config = Config {
            ipfs_gateway: "https://ipfs.io".to_string(),
            timeout_secs: 5,
            rpc_url: String::new(),
            horizon_url: String::new(),
            contract_id: String::new(),
            oracle_secret_key: String::new(),
            network_passphrase: String::new(),
        };

        // Use a CID that definitely doesn't exist
        let result = fetch_and_hash_proof("QmInvalidCIDThatDoesNotExist123456789", &config).await;

        assert!(result.is_err());
        match result {
            Err(OracleError::ProofNotFound(_)) | Err(OracleError::Network(_)) => {}
            _ => panic!("Expected ProofNotFound or Network error"),
        }
    }
}
