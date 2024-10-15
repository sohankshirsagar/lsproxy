use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    PyrightClient, RustAnalyzerClient, TypeScriptLanguageClient, PYRIGHT_FILE_PATTERNS,
    RUST_ANALYZER_FILE_PATTERNS, TYPESCRIPT_FILE_PATTERNS,
};
use crate::lsp::types::SupportedLSP;
use crate::lsp::DEFAULT_EXCLUDE_PATTERNS;
use crate::utils::file_utils::search_files;
use log::{debug, warn};
use lsp_types::{
    DocumentSymbolResponse, GotoDefinitionResponse, Location, Position, WorkspaceSymbolResponse,
};
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct LspManager {
    clients: HashMap<SupportedLSP, Arc<Mutex<Box<dyn LspClient>>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    fn detect_languages(&self, root_path: &str) -> Vec<SupportedLSP> {
        let mut lsps = Vec::new();
        for lsp in [
            SupportedLSP::Python,
            SupportedLSP::TypeScriptJavaScript,
            SupportedLSP::Rust,
        ] {
            let patterns = match lsp {
                SupportedLSP::Python => PYRIGHT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLSP::TypeScriptJavaScript => TYPESCRIPT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLSP::Rust => RUST_ANALYZER_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
            };
            if search_files(
                Path::new(root_path),
                patterns,
                DEFAULT_EXCLUDE_PATTERNS
                    .iter()
                    .map(|s| s.to_string())
                    .collect(),
            )
            .map_err(|e| warn!("Error searching files: {}", e))
            .unwrap_or_default()
            .len()
                > 0
            {
                lsps.push(lsp);
            }
        }
        debug!("Starting LSPs: {:?}", lsps);
        lsps
    }

    pub async fn start_langservers(&mut self, repo_path: &str) -> Result<(), String> {
        let lsps = self.detect_languages(repo_path);
        for lsp in lsps {
            if self.get_client(lsp).is_some() {
                continue;
            }
            debug!("Starting {:?} LSP", lsp);
            let mut client: Box<dyn LspClient> = match lsp {
                SupportedLSP::Python => Box::new(
                    PyrightClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLSP::TypeScriptJavaScript => Box::new(
                    TypeScriptLanguageClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLSP::Rust => Box::new(
                    RustAnalyzerClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
            };
            client
                .initialize(repo_path.to_string())
                .await
                .map_err(|e| e.to_string())?;
            client
                .setup_workspace(repo_path)
                .await
                .map_err(|e| e.to_string())?;
            self.clients.insert(lsp, Arc::new(Mutex::new(client)));
        }
        Ok(())
    }

    pub async fn file_symbols(
        &self,
        file_path: &str,
    ) -> Result<DocumentSymbolResponse, Box<dyn std::error::Error + Send + Sync>> {
        let lsp_type = self.detect_language(&file_path)?;
        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;
        let mut locked_client = client.lock().await;
        locked_client.text_document_symbols(file_path).await
    }

    pub async fn get_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let lsp_type = self.detect_language(file_path)?;

        if let Some(client) = self.get_client(lsp_type) {
            let mut locked_client = client.lock().await;
            locked_client
                .text_document_definition(file_path, position)
                .await
        } else {
            warn!("No LSP client found for file type {:?}", lsp_type);
            Err("No LSP client found for file type".into())
        }
    }

    pub async fn workspace_symbols(
        &self,
        query: &str,
    ) -> Result<Vec<WorkspaceSymbolResponse>, Box<dyn std::error::Error + Send + Sync>> {
        /* This returns results for all langservers*/
        let mut symbols = Vec::new();
        for client in self.clients.values() {
            let mut locked_client = client.lock().await;
            let client_symbols = locked_client.workspace_symbols(query).await?;
            symbols.push(client_symbols);
        }
        Ok(symbols)
    }

    pub fn get_client(&self, lsp_type: SupportedLSP) -> Option<Arc<Mutex<Box<dyn LspClient>>>> {
        self.clients.get(&lsp_type).cloned()
    }

    pub async fn get_references(
        &self,
        file_path: &str,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        let lsp_type = self.detect_language(file_path)?;
        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;
        let mut locked_client = client.lock().await;
        locked_client
            .text_document_reference(file_path, position, include_declaration)
            .await
    }

    fn detect_language(
        &self,
        file_path: &str,
    ) -> Result<SupportedLSP, Box<dyn Error + Send + Sync>> {
        let path: PathBuf = PathBuf::from(file_path);
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("py") => Ok(SupportedLSP::Python),
            Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
                Ok(SupportedLSP::TypeScriptJavaScript)
            }
            Some("rs") => Ok(SupportedLSP::Rust),
            _ => Err("Unsupported file type".into()),
        }
    }
}
