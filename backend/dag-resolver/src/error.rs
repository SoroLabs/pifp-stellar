use thiserror::Error;

pub type Result<T> = std::result::Result<T, DagResolutionError>;

#[derive(Debug, Error)]
pub enum DagResolutionError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("empty intent set")]
    EmptyInput,
    #[error("duplicate intent identifier: {0}")]
    DuplicateIntentId(String),
    #[error("invalid intent '{id}': {reason}")]
    InvalidIntent { id: String, reason: String },
    #[error("unknown dependency '{dependency}' referenced by intent '{intent}'")]
    UnknownDependency { intent: String, dependency: String },
    #[error("cycle detected: {0}")]
    CycleDetected(String),
}
