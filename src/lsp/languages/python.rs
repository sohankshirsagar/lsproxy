use std::process::Stdio;

use async_trait::async_trait;
use tokio::process::Command;

use crate::lsp::{JsonRpcHandler, LspClient, ProcessHandler};

pub const PYRIGHT_ROOT_FILES: &[&str] = &[
    ".git",
    "pyproject.toml",
    "setup.py",
    "setup.cfg",
    "requirements.txt",
    "Pipfile",
    "pyrightconfig.json",
];

pub const PYRIGHT_FILE_PATTERNS: &[&str] = &["**/*.py"];

pub struct PyrightClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

#[async_trait]
impl LspClient for PyrightClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        PYRIGHT_ROOT_FILES.iter().map(|&s| s.to_string()).collect()
    }

    fn get_file_patterns(&mut self) -> Vec<String> {
        PYRIGHT_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect()
    }
}

impl PyrightClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("pyright-langserver")
            .arg("--stdio")
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
