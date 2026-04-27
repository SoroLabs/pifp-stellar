use thiserror::Error;

pub type Result<T> = std::result::Result<T, FormalVerificationError>;

#[derive(Debug, Error)]
pub enum FormalVerificationError {
    #[error("WASM parse error: {0}")]
    WasmParse(String),
    #[error("unsupported opcode at byte offset {offset}: {opcode}")]
    UnsupportedOpcode { offset: usize, opcode: String },
    #[error("symbolic execution error: {0}")]
    Symbolic(String),
    #[error("solver error: {0}")]
    Solver(String),
    #[error("upgrade blocked: {0}")]
    Blocked(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
