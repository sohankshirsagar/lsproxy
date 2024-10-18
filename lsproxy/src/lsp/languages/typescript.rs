use std::{
    collections::HashMap,
    error::Error,
    fs::read_to_string,
    path::Path,
    process::Stdio,
    sync::{Arc, Mutex},
};

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
    workspace_files_cache: Arc<Mutex<Option<HashMap<String, Option<String>>>>>,
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

    fn get_workspace_files_cache(&mut self) -> Arc<Mutex<Option<HashMap<String, Option<String>>>>> {
        self.workspace_files_cache.clone()
    }

    fn set_workspace_files_cache(&mut self, cache: HashMap<String, Option<String>>) {
        self.workspace_files_cache = Arc::new(Mutex::new(Some(cache)));
    }

    fn get_include_patterns(&mut self) -> Vec<String> {
        TYPESCRIPT_FILE_PATTERNS
            .iter()
            .map(|&s| s.to_string())
            .collect()
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

        let mut cache: HashMap<String, Option<String>> = HashMap::new();

        let text_document_items = self.get_text_document_items_to_open(root_path).await?;
        for item in text_document_items {
            cache.insert(
                item.uri
                    .to_file_path()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string(),
                Some(item.text.to_owned()),
            );
            self.text_document_did_open(item).await?;
        }
        self.set_workspace_files_cache(cache);
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
            workspace_files_cache: Arc::new(Mutex::new(None)),
        })
    }

    pub async fn get_text_document_items_to_open(
        &mut self,
        workspace_path: &str,
    ) -> Result<Vec<TextDocumentItem>, Box<dyn Error + Send + Sync>> {
        let tsconfig_path = Path::new(workspace_path).join("tsconfig.json");
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
            Path::new(workspace_path),
            include_patterns.into_iter().map(String::from).collect(),
            exclude_patterns.into_iter().map(String::from).collect(),
        )?;

        futures::future::try_join_all(files.into_iter().map(|file_path| async move {
            let content = self
                .read_text_document(file_path.to_str().ok_or("Invalid file path")?, None)
                .await
                .unwrap();
            Ok(TextDocumentItem {
                uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                language_id: "typescript".to_string(),
                version: 1,
                text: content,
            })
        }))
        .await
    }
}
