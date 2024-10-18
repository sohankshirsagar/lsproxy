use std::{error::Error, fs::read_to_string, path::Path, process::Stdio};

use async_trait::async_trait;
use log::debug;
use lsp_types::TextDocumentItem;
use serde_json::{from_str, Value};
use tokio::process::Command;
use url::Url;

use crate::lsp::{
    workspace_documents::{WorkspaceDocuments, WorkspaceDocumentsHandler},
    JsonRpcHandler, LspClient, ProcessHandler, DEFAULT_EXCLUDE_PATTERNS,
};

pub struct TypeScriptLanguageClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
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

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        /*
        The server needs to know about all the files in the workspace to provide features like go to definition.
        This is a limitation of the tsserver    .
         */
        debug!("Setting up workspace for TypeScript client");

        let text_document_items = self
            .get_text_document_items_to_open_with_config(root_path)
            .await?;
        for item in text_document_items {
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
        let workspace_documents = WorkspaceDocumentsHandler::new(
            root_path,
            TYPESCRIPT_FILE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
        );
        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents: workspace_documents,
        })
    }

    pub async fn get_text_document_items_to_open_with_config(
        &mut self,
        workspace_path: &str,
    ) -> Result<Vec<TextDocumentItem>, Box<dyn Error + Send + Sync>> {
        let tsconfig_path = Path::new(workspace_path).join("tsconfig.json");
        let tsconfig_content = read_to_string(tsconfig_path).unwrap_or_else(|_| "{}".to_string());
        let tsconfig: Value = from_str(&tsconfig_content)?;

        let include_patterns = tsconfig["include"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_else(|| TYPESCRIPT_FILE_PATTERNS.to_vec());
        let exclude_patterns = tsconfig["exclude"]
            .as_array()
            .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
            .unwrap_or_else(|| DEFAULT_EXCLUDE_PATTERNS.to_vec());
        let workspace_documents = self.get_workspace_documents();
        workspace_documents
            .update_patterns(
                include_patterns
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
                exclude_patterns
                    .into_iter()
                    .map(|s| s.to_string())
                    .collect(),
            )
            .await;
        let file_paths = workspace_documents.list_files().await;
        let mut items = Vec::with_capacity(file_paths.len());
        for file_path in file_paths {
            let content = match workspace_documents.read_text_document(&file_path, None).await {
                Ok(content) => content,
                Err(e) => {
                    debug!("Failed to read document {}: {}", file_path, e);
                    return Err(e);
                }
            };
            let item = TextDocumentItem {
                uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                language_id: "typescript".to_string(),
                version: 1,
                text: content,
            };
            items.push(item);
        }
        Ok(items)
    }
}
