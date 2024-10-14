use thiserror::Error;

#[derive(Error, Debug)]
pub enum LspClientError {
    #[error("IO Error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON Parsing Error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("JSON-RPC Error: {0}")]
    JsonRpc(String),

    #[error("Timeout while waiting for response")]
    Timeout,

    #[error("Unexpected EOF from process")]
    UnexpectedEof,

    #[error("Other Error: {0}")]
    Other(String),
}
