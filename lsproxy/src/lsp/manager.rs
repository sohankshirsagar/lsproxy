use crate::api_types::SupportedLanguages;
use crate::lsp::client::LspClient;
use crate::lsp::languages::{
    PyrightClient, RustAnalyzerClient, TypeScriptLanguageClient, PYRIGHT_FILE_PATTERNS,
    RUST_ANALYZER_FILE_PATTERNS, TYPESCRIPT_FILE_PATTERNS,
};
use crate::lsp::DEFAULT_EXCLUDE_PATTERNS;
use crate::utils::file_utils::search_files;
use log::{debug, error, warn};
use lsp_types::{
    DocumentSymbolResponse, GotoDefinitionResponse, Location, Position, WorkspaceSymbolResponse,
};
use std::collections::HashMap;
use std::error::Error;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct LspManager {
    clients: HashMap<SupportedLanguages, Arc<Mutex<Box<dyn LspClient>>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    fn detect_languages(&self, root_path: &str) -> Vec<SupportedLanguages> {
        let mut lsps = Vec::new();
        for lsp in [
            SupportedLanguages::Python,
            SupportedLanguages::TypeScriptJavaScript,
            SupportedLanguages::Rust,
        ] {
            let patterns = match lsp {
                SupportedLanguages::Python => PYRIGHT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::TypeScriptJavaScript => TYPESCRIPT_FILE_PATTERNS
                    .iter()
                    .map(|&s| s.to_string())
                    .collect(),
                SupportedLanguages::Rust => RUST_ANALYZER_FILE_PATTERNS
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
                SupportedLanguages::Python => Box::new(
                    PyrightClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::TypeScriptJavaScript => Box::new(
                    TypeScriptLanguageClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLanguages::Rust => Box::new(
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
        let document_symbol_response = match locked_client.text_document_symbols(file_path).await {
            Ok(response) => response,
            Err(e) => return Err(Box::new(e) as Box<dyn std::error::Error + Send + Sync>),
        };
        Ok(document_symbol_response)
    }

    pub async fn definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let lsp_type = self.detect_language(file_path)?;
        if let Some(client) = self.get_client(lsp_type) {
            let mut locked_client = client.lock().await;
            let lsp_response = locked_client
                .text_document_definition(file_path, position)
                .await?;

            // Convert the LSP response to our custom type
            Ok(lsp_response)
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
        for (lang, client) in self.clients.iter() {
            debug!(
                "Requesting workspace symbols with query: {} for {:?}",
                query, lang
            );
            let mut locked_client = client.lock().await;
            let client_symbols = locked_client.workspace_symbols(query).await;
            match client_symbols {
                Ok(response) => symbols.push(response),
                Err(e) => error!("Error requesting workspace symbols for {:?}: {:?}. Continuing...", lang, e),
            }
        }
        Ok(symbols)
    }

    pub fn get_client(
        &self,
        lsp_type: SupportedLanguages,
    ) -> Option<Arc<Mutex<Box<dyn LspClient>>>> {
        self.clients.get(&lsp_type).cloned()
    }

    pub async fn references(
        &self,
        file_path: &str,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        let lsp_type = self.detect_language(file_path)?;
        let client = self.get_client(lsp_type).ok_or_else(|| format!("LSP client not found for {:?}", lsp_type))?;
        let mut locked_client = client.lock().await;

        let locations = locked_client
            .text_document_reference(file_path, position, include_declaration)
            .await?;

        Ok(locations)
    }

    fn detect_language(
        &self,
        file_path: &str,
    ) -> Result<SupportedLanguages, Box<dyn Error + Send + Sync>> {
        let path: PathBuf = PathBuf::from(file_path);
        match path.extension().and_then(|ext| ext.to_str()) {
            Some("py") => Ok(SupportedLanguages::Python),
            Some("js") | Some("ts") | Some("jsx") | Some("tsx") => {
                Ok(SupportedLanguages::TypeScriptJavaScript)
            }
            Some("rs") => Ok(SupportedLanguages::Rust),
            _ => Err("Unsupported file type".into()),
        }
    }
}
