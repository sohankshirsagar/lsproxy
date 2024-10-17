use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::lsp::{JsonRpcHandler, LspClient, ProcessHandler};

pub struct GoplsClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

pub const GOLANG_ROOT_FILES: &[&str] = &["go.mod","go.work"];
pub const GOLANG_FILE_PATTERNS: &[&str] = &["**/*.go"];

#[async_trait]
impl LspClient for GoplsClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        GOLANG_ROOT_FILES
            .iter()
            .map(|&s| s.to_owned())
            .collect()
    }
}

impl GoplsClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("gopls")
            .arg("-mode=stdio")
            .arg("-vv")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
        })
    }
}
