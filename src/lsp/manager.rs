use crate::lsp::client::{LspClient, PythonClient};
use crate::lsp::types::{SupportedLSP, UniqueDefinition};
use crate::lsp::{JsonRpcHandler, ProcessHandler, TypeScriptClient};
use crate::utils::{find_symbol_occurrences, get_files_for_workspace_typescript};
use log::{debug, error, info, warn};
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, InitializeResult};
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fs::read_dir;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
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
                    PythonClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLSP::TypeScriptJavaScript => Box::new(
                    TypeScriptClient::new(repo_path)
                        .await
                        .map_err(|e| e.to_string())?,
                ),
                SupportedLSP::Rust => return Err("Rust LSP not implemented yet".to_string()),
                _ => return Err(format!("Unsupported LSP {:?}", lsp)),
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

    async fn initialize_client(
        &mut self,
        lsp_type: SupportedLSP,
        repo_path: String,
    ) -> Result<InitializeResult, Box<dyn std::error::Error + Send + Sync>> {
        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;
        let mut locked_client = client.lock().await;
        locked_client.initialize(repo_path).await
    }

    pub async fn get_symbols(
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
        symbol_name: &str,
    ) -> Result<Vec<GotoDefinitionResponse>, Box<dyn std::error::Error + Send + Sync>> {
        let mut unique_definitions = HashSet::new();
        let lsp_type = self.detect_language(file_path)?;

        if let Some(client) = self.get_client(lsp_type) {
            let occurrences = find_symbol_occurrences(file_path, symbol_name)?;

            if occurrences.is_empty() {
                info!(
                    "No occurrences of symbol '{}' found in file '{}'",
                    symbol_name, file_path
                );
                return Ok(vec![]);
            }

            info!(
                "Found {} occurrences of symbol '{}' in file '{}'",
                occurrences.len(),
                symbol_name,
                file_path
            );

            let mut locked_client = client.lock().await;

            for occurrence in occurrences {
                match locked_client
                    .text_document_definition(
                        file_path,
                        occurrence.start_line as u32 - 1,
                        occurrence.start_column as u32 - 1,
                    )
                    .await
                {
                    Ok(definition) => {
                        info!(
                            "Found definition for symbol '{}' at line {}, column {}",
                            symbol_name, occurrence.start_line, occurrence.start_column
                        );
                        // Insert the UniqueDefinition into the HashSet
                        unique_definitions.insert(UniqueDefinition::from(definition));
                    }
                    Err(e) => {
                        warn!(
                            "Failed to get definition for symbol '{}' at line {}, column {}: {}",
                            symbol_name, occurrence.start_line, occurrence.start_column, e
                        );
                    }
                }
            }
        } else {
            warn!("No LSP client found for file type {:?}", lsp_type);
        }

        let unique_definitions_vec: Vec<GotoDefinitionResponse> = unique_definitions
            .into_iter()
            .map(|ud| ud.original)
            .collect();

        if unique_definitions_vec.is_empty() {
            info!("No unique definitions found for symbol '{}'", symbol_name);
        } else {
            info!(
                "Found {} unique definition(s) for symbol '{}'",
                unique_definitions_vec.len(),
                symbol_name
            );
        }

        Ok(unique_definitions_vec)
    }

    async fn find_python_root(&mut self, repo_path: &str) -> String {
        repo_path.to_string()
    }

    async fn find_typescript_root(&mut self, repo_path: &str) -> String {
        if let Some(first_tsconfig) = self.find_tsconfig_files(repo_path).first() {
            if let Some(parent) = first_tsconfig.parent() {
                debug!(
                    "Found tsconfig at {}",
                    parent.to_string_lossy().into_owned()
                );
                return parent.to_string_lossy().into_owned();
            }
        }
        debug!("Didn't find tsconfig");
        repo_path.to_string()
    }

    fn find_tsconfig_files(&self, dir: &str) -> Vec<PathBuf> {
        let mut result = Vec::new();
        if let Ok(entries) = read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(file_name) = path.file_name() {
                        if file_name != "node_modules" {
                            result.extend(
                                self.find_tsconfig_files(&path.to_string_lossy().into_owned()),
                            );
                        }
                    }
                } else if path.file_name().unwrap() == "tsconfig.json" {
                    result.push(path);
                }
            }
        }
        result
    }

    async fn find_rust_root(&mut self, repo_path: &str) -> String {
        //TODO Actually find and verify
        repo_path.to_string()
    }

    pub fn get_client(&self, lsp_type: SupportedLSP) -> Option<Arc<Mutex<Box<dyn LspClient>>>> {
        self.clients.get(&lsp_type).cloned()
    }

    fn detect_language(
        &self,
        file_path: &str,
    ) -> Result<SupportedLSP, Box<dyn Error + Send + Sync>> {
        // Open the file
        let path = PathBuf::from(file_path);
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
