//! Application-wide error types.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum IndexerError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Migration error: {0}")]
    Migrate(#[from] sqlx::migrate::MigrateError),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Event parse error: {0}")]
    EventParse(String),
}

pub type Result<T> = std::result::Result<T, IndexerError>;
