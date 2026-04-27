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

impl axum::response::IntoResponse for OracleError {
    fn into_response(self) -> axum::response::Response {
        let (status, error_message) = match self {
            OracleError::Config(s) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, s),
            OracleError::Network(s) => (axum::http::StatusCode::BAD_GATEWAY, s),
            OracleError::ProofNotFound(s) => (axum::http::StatusCode::NOT_FOUND, s),
            OracleError::Verification(s) => (axum::http::StatusCode::UNPROCESSABLE_ENTITY, s),
            OracleError::Transaction(s) => (axum::http::StatusCode::BAD_REQUEST, s),
            OracleError::ContractError(s) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, s),
            OracleError::Io(s) => (axum::http::StatusCode::INTERNAL_SERVER_ERROR, s.to_string()),
        };

        let body = axum::Json(serde_json::json!({
            "error": error_message,
        }));

        (status, body).into_response()
    }
}

