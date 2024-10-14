use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use uuid::Uuid;

pub trait JsonRpc: Send + Sync {
    fn create_request(&self, method: &str, params: Value) -> String;
    fn create_notification(&self, method: &str, params: Value) -> String;
    fn parse_message(&self, data: &str) -> Result<JsonRpcMessage, JsonRpcError>;
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcMessage {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: Option<String>,
    pub params: Option<Value>,
    pub result: Option<Value>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    pub data: Option<Value>,
}

impl fmt::Display for JsonRpcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Error {}: {}", self.code, self.message)
    }
}

impl std::error::Error for JsonRpcError {}

pub struct JsonRpcHandler;

impl JsonRpcHandler {
    pub fn new() -> Self {
        Self
    }
}

impl JsonRpc for JsonRpcHandler {
    fn create_request(&self, method: &str, params: Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": Uuid::new_v4().to_string(),
            "method": method,
            "params": params
        })
        .to_string()
    }

    fn create_notification(&self, method: &str, params: Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        })
        .to_string()
    }

    fn parse_message(&self, data: &str) -> Result<JsonRpcMessage, JsonRpcError> {
        serde_json::from_str(data).map_err(|e| JsonRpcError {
            code: -32700,
            message: e.to_string(),
            data: None,
        })
    }
}
