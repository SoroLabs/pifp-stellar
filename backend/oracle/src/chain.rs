//! Soroban chain interaction logic.
//!
//! Builds and submits verify_and_release transactions to the PIFP contract.

use serde_json::json;
use tracing::{debug, info, warn};

use crate::config::Config;
use crate::errors::{OracleError, Result};

/// Submit a verify_and_release transaction to the PIFP contract.
///
/// # Arguments
/// * `config` - Oracle configuration
/// * `project_id` - Project ID to verify
/// * `proof_hash` - 32-byte SHA-256 hash of the proof artifact
///
/// # Returns
/// Transaction hash on success
///
/// # Errors
/// Returns error if:
/// - Transaction simulation fails (e.g., project already completed)
/// - Transaction submission fails
/// - Network errors occur
pub async fn submit_verification(
    config: &Config,
    project_id: u64,
    proof_hash: [u8; 32],
) -> Result<String> {
    info!(
        "Building verify_and_release transaction for project {}",
        project_id
    );

    // Decode oracle secret key
    let oracle_keypair = decode_secret_key(&config.oracle_secret_key)?;
    let oracle_address = keypair_to_address(&oracle_keypair);

    debug!("Oracle address: {}", oracle_address);

    // Build transaction parameters
    let params = build_transaction_params(
        &config.contract_id,
        &oracle_address,
        project_id,
        &proof_hash,
    )?;

    // Step 1: Simulate transaction to check for errors
    info!("Simulating transaction...");
    simulate_transaction(config, &params).await?;

    // Step 2: Submit transaction to network
    info!("Submitting transaction to network...");
    let tx_hash = submit_transaction(config, &params, &oracle_keypair).await?;

    Ok(tx_hash)
}

/// Build transaction parameters for verify_and_release invocation.
fn build_transaction_params(
    contract_id: &str,
    oracle_address: &str,
    project_id: u64,
    proof_hash: &[u8; 32],
) -> Result<serde_json::Value> {
    // Convert proof hash to hex string for Soroban RPC
    let proof_hash_hex = hex::encode(proof_hash);

    Ok(json!({
        "contractId": contract_id,
        "function": "verify_and_release",
        "args": [
            {
                "address": oracle_address
            },
            {
                "u64": project_id
            },
            {
                "bytes": proof_hash_hex
            }
        ]
    }))
}

/// Simulate transaction to detect errors before submission.
///
/// This catches common errors like:
/// - Project not found
/// - Project already completed
/// - Proof hash mismatch
/// - Oracle not authorized
async fn simulate_transaction(config: &Config, params: &serde_json::Value) -> Result<()> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| OracleError::Network(format!("Failed to create HTTP client: {e}")))?;

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "simulateTransaction",
        "params": params
    });

    let response = client
        .post(&config.rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| OracleError::Network(format!("Simulation request failed: {e}")))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| OracleError::Network(format!("Failed to parse simulation response: {e}")))?;

    debug!(
        "Simulation response: {}",
        serde_json::to_string_pretty(&response_json).unwrap()
    );

    // Check for RPC errors
    if let Some(error) = response_json.get("error") {
        return Err(OracleError::Transaction(format!(
            "Simulation failed: {error}"
        )));
    }

    // Check for contract errors in result
    if let Some(result) = response_json.get("result") {
        if let Some(error) = result.get("error") {
            // Parse contract error
            let error_msg = parse_contract_error(error);
            return Err(OracleError::ContractError(error_msg));
        }
    }

    info!("✓ Transaction simulation successful");
    Ok(())
}

/// Submit the signed transaction to the network.
async fn submit_transaction(
    config: &Config,
    params: &serde_json::Value,
    _keypair: &str,
) -> Result<String> {
    // NOTE: This is a simplified implementation.
    // In production, you would:
    // 1. Build the full XDR transaction envelope
    // 2. Sign it with the oracle keypair
    // 3. Submit via Horizon's /transactions endpoint
    //
    // For now, we'll use the Soroban RPC's sendTransaction method
    // which handles signing internally (requires stellar-cli integration)

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.timeout_secs))
        .build()
        .map_err(|e| OracleError::Network(format!("Failed to create HTTP client: {e}")))?;

    let request_body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "sendTransaction",
        "params": params
    });

    let response = client
        .post(&config.rpc_url)
        .json(&request_body)
        .send()
        .await
        .map_err(|e| OracleError::Network(format!("Transaction submission failed: {e}")))?;

    let response_json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| OracleError::Network(format!("Failed to parse submission response: {e}")))?;

    debug!(
        "Submission response: {}",
        serde_json::to_string_pretty(&response_json).unwrap()
    );

    // Check for errors
    if let Some(error) = response_json.get("error") {
        return Err(OracleError::Transaction(format!(
            "Transaction submission failed: {error}"
        )));
    }

    // Extract transaction hash from result
    let tx_hash = response_json
        .get("result")
        .and_then(|r| r.get("hash"))
        .and_then(|h| h.as_str())
        .ok_or_else(|| OracleError::Transaction("No transaction hash in response".to_string()))?;

    Ok(tx_hash.to_string())
}

/// Parse contract error from simulation response.
fn parse_contract_error(error: &serde_json::Value) -> String {
    // Try to extract error code and message
    if let Some(code) = error.get("code") {
        let code_str = code
            .as_u64()
            .map(|c| c.to_string())
            .or_else(|| code.as_str().map(|s| s.to_string()))
            .unwrap_or_else(|| "unknown".to_string());

        // Map known error codes to human-readable messages
        let message = match code_str.as_str() {
            "1" => "Project not found",
            "3" => "Project already completed (milestone already released)",
            "6" => "Not authorized (oracle role required)",
            "14" => "Project expired",
            "16" => "Verification failed (proof hash mismatch)",
            _ => "Unknown contract error",
        };

        return format!("{message} (code: {code_str})");
    }

    error.to_string()
}

/// Decode a Stellar secret key from Strkey format.
fn decode_secret_key(secret_key: &str) -> Result<String> {
    // Validate format
    if !secret_key.starts_with('S') {
        return Err(OracleError::Config(
            "Invalid secret key format (must start with 'S')".to_string(),
        ));
    }

    // In production, use stellar-strkey crate to decode
    // For now, return as-is for the mock implementation
    Ok(secret_key.to_string())
}

/// Convert keypair to Stellar address (G...).
fn keypair_to_address(keypair: &str) -> String {
    // In production, derive the public key from the secret key
    // For now, return a placeholder
    // This would use stellar-strkey to decode the secret key and derive the public key

    warn!("Using mock address derivation - implement proper key derivation in production");
    format!("G{}", &keypair[1..]) // Mock: just replace S with G
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_transaction_params() {
        let contract_id = "CAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD2KM";
        let oracle_address = "GAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA";
        let project_id = 42;
        let proof_hash = [0u8; 32];

        let params =
            build_transaction_params(contract_id, oracle_address, project_id, &proof_hash).unwrap();

        assert_eq!(params["contractId"], contract_id);
        assert_eq!(params["function"], "verify_and_release");
        assert_eq!(params["args"][1]["u64"], 42);
    }

    #[test]
    fn test_parse_contract_error_known_code() {
        let error = json!({"code": 3});
        let message = parse_contract_error(&error);
        assert!(message.contains("already completed"));
    }

    #[test]
    fn test_parse_contract_error_unknown_code() {
        let error = json!({"code": 999});
        let message = parse_contract_error(&error);
        assert!(message.contains("Unknown"));
    }

    #[test]
    fn test_decode_secret_key_valid() {
        let result = decode_secret_key("SAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA");
        assert!(result.is_ok());
    }

    #[test]
    fn test_decode_secret_key_invalid() {
        let result = decode_secret_key("INVALID");
        assert!(result.is_err());
    }
}
