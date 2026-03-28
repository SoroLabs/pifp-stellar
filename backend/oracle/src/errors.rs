//! Error types for the Oracle service.

use thiserror::Error;

pub type Result<T> = std::result::Result<T, OracleError>;

#[derive(Error, Debug)]
pub enum OracleError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Proof not found: {0}")]
    ProofNotFound(String),

    #[error("Verification error: {0}")]
    Verification(String),

    #[error("Transaction error: {0}")]
    Transaction(String),

    #[error("Contract error: {0}")]
    ContractError(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
