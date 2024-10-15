use crate::lsp::client::LspClient;
use crate::lsp::languages::{PyrightClient, RustAnalyzerClient, TypeScriptLanguageClient};
use crate::lsp::types::{CustomDocumentSymbolResponse, CustomGotoDefinitionResponse, CustomReferenceResponse, SupportedLSP};
use log::{debug, warn};
use lsp_types::{Position, WorkspaceSymbolResponse
};
use std::collections::HashMap;
use std::error::Error;
use std::path::PathBuf;
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

    pub async fn start_langservers(
        &mut self,
        repo_path: &str,
        lsps: &[SupportedLSP],
    ) -> Result<(), String> {
        for &lsp in lsps {
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
    ) -> Result<CustomDocumentSymbolResponse, Box<dyn std::error::Error + Send + Sync>> {
        let lsp_type = self.detect_language(&file_path)?;
        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;
        let mut locked_client = client.lock().await;
        let document_symbol_response = locked_client.text_document_symbols(file_path).await?;
        let custom_document_symbol_response = CustomDocumentSymbolResponse::from(document_symbol_response);
        Ok(custom_document_symbol_response)
    }

    pub async fn get_definition(
        &self,
        file_path: &str,
        position: Position,
    ) -> Result<CustomGotoDefinitionResponse, Box<dyn std::error::Error + Send + Sync>> {
        let lsp_type = self.detect_language(file_path)?;
        if let Some(client) = self.get_client(lsp_type) {
            let mut locked_client = client.lock().await;
            let lsp_response = locked_client
                .text_document_definition(file_path, position)
                .await?;

            // Convert the LSP response to our custom type
            Ok(CustomGotoDefinitionResponse::from(lsp_response))
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
    ) -> Result<CustomReferenceResponse, Box<dyn Error + Send + Sync>> {
        let lsp_type = self.detect_language(file_path)?;
        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;
        let mut locked_client = client.lock().await;

        let locations = locked_client
            .text_document_reference(file_path, position, include_declaration)
            .await?;

        Ok(CustomReferenceResponse::from(locations))
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
