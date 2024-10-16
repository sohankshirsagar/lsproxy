use std::{error::Error, fs::read_to_string, path::Path, process::Stdio};

use async_trait::async_trait;
use log::debug;
use lsp_types::TextDocumentItem;
use serde_json::Value;
use tokio::process::Command;
use url::Url;

use crate::{
    lsp::{JsonRpcHandler, LspClient, ProcessHandler, DEFAULT_EXCLUDE_PATTERNS},
    utils::file_utils::search_files,
};

pub struct TypeScriptLanguageClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

pub const TYPESCRIPT_ROOT_FILES: &[&str] =
    &["tsconfig.json", "jsconfig.json", "package.json", ".git"];

pub const TYPESCRIPT_FILE_PATTERNS: &[&str] = &["**/*.ts", "**/*.tsx", "**/*.js", "**/*.jsx"];

#[async_trait]
impl LspClient for TypeScriptLanguageClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        TYPESCRIPT_ROOT_FILES
            .iter()
            .map(|&s| s.to_owned())
            .collect()
    }

    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        /*
        The server needs to know about all the files in the workspace to provide features like go to definition.
        This is a limitation of the TypeScript language server.
         */
        debug!("Setting up workspace for TypeScript client");
        let text_document_items =
            TypeScriptLanguageClient::get_text_document_items_to_open(root_path).unwrap();
        for item in text_document_items {
            debug!("Sent 'didOpen' for file: {}", item.uri.to_string());
            self.text_document_did_open(item).await?;
        }
        debug!("Workspace setup completed for TypeScript client");
        Ok(())
    }
}

impl TypeScriptLanguageClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("typescript-language-server")
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

    pub fn get_text_document_items_to_open(
        repo_path: &str,
    ) -> Result<Vec<TextDocumentItem>, Box<dyn std::error::Error>> {
        let tsconfig_path = Path::new(repo_path).join("tsconfig.json");
        let tsconfig_content = read_to_string(tsconfig_path).unwrap_or_else(|_| "{}".to_string());
        let tsconfig: Value = serde_json::from_str(&tsconfig_content)?;

        let include_patterns = tsconfig["include"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_else(|| TYPESCRIPT_FILE_PATTERNS.to_vec());
        let exclude_patterns = tsconfig["exclude"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_else(|| DEFAULT_EXCLUDE_PATTERNS.to_vec());

        let files = search_files(
            Path::new(repo_path),
            include_patterns.into_iter().map(String::from).collect(),
            exclude_patterns.into_iter().map(String::from).collect(),
        )?;

        files
            .into_iter()
            .map(|file_path| {
                let content = read_to_string(&file_path)?;
                Ok(TextDocumentItem {
                    uri: Url::from_file_path(&file_path).map_err(|_| "Invalid file path")?,
                    language_id: "typescript".to_string(),
                    version: 1,
                    text: content,
                })
            })
            .collect()
    }
}
