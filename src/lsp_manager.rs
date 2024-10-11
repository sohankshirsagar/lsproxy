use crate::lsp_client::LspClient;
use crate::symbol_finder::python_symbol_finder;
use crate::types::{SupportedLSPs, UniqueDefinition};
use log::{error, info, warn};
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, InitializeResult, Location};
use std::collections::{HashMap, HashSet};
use std::fs::{File, read_dir};
use std::io::Read;
use std::path::PathBuf;
use std::process::Stdio;
use std::sync::Arc;
use tokio::process::Command;
use tokio::sync::Mutex;

pub struct LspManager {
    clients: HashMap<SupportedLSPs, Arc<Mutex<LspClient>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub async fn start_lsps(
        &mut self,
        repo_path: &str,
        lsps: &[SupportedLSPs],
    ) -> Result<(), String> {
        for &lsp in lsps {
            let result = match lsp {
                SupportedLSPs::Python => self.start_python_lsp(repo_path).await,
                SupportedLSPs::TypeScript => self.start_typescript_lsp(repo_path).await,
                SupportedLSPs::Rust => self.start_rust_lsp(repo_path).await,
            };
            if let Err(e) = result {
                error!("Failed to start {:?} LSP: {}", lsp, e);
                return Err(format!("Failed to start {:?} LSP: {}", lsp, e));
            }
        }
        Ok(())
    }

    async fn create_client(
        &mut self,
        lsp_type: SupportedLSPs,
        process: tokio::process::Child,
    ) -> Result<(), String> {
        // Await the async function LspClient::new
        let client = LspClient::new(process)
            .await
            .map_err(|e| format!("Failed to create LSP client: {}", e))?;
        let client = Arc::new(Mutex::new(client));
        self.clients.insert(lsp_type, client.clone());
        Ok(())
    }

    async fn initialize_client(
        &mut self,
        lsp_type: SupportedLSPs,
        repo_path: String,
    ) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;

        // Initialize the client
        let mut locked_client = client.lock().await;
        locked_client.initialize(Some(repo_path.clone())).await
    }

    pub async fn get_symbols(
        &self,
        file_path: &str,
    ) -> Result<DocumentSymbolResponse, Box<dyn std::error::Error>> {
        // Detect the language
        let lsp_type = self.detect_language(&file_path)?;

        let client = self.get_client(lsp_type).ok_or("LSP client not found")?;

        // Call get_symbols on the LSP client
        let mut locked_client = client.lock().await;
        locked_client.get_symbols(file_path).await
    }

    pub async fn get_definition(
        &self,
        file_path: &str,
        symbol_name: &str,
    ) -> Result<Vec<GotoDefinitionResponse>, Box<dyn std::error::Error>> {
        let mut unique_definitions = HashSet::new();
        let lsp_type = self.detect_language(file_path)?;

        if let Some(client) = self.get_client(lsp_type) {
            let occurrences =
                python_symbol_finder::find_symbol_occurrences(file_path, symbol_name)?;

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
                    .get_definition(
                        file_path,
                        occurrence.start_line as u32 - 1, // LSP uses 0-based line numbers
                        occurrence.start_column as u32 - 1, // LSP uses 0-based column numbers
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

    async fn start_python_lsp(
        &mut self,
        repo_path: &str,
    ) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        let python_path = self.find_python_root(repo_path).await;
        // Spawn the LSP server using tokio's async process
        let process = Command::new("pyright-langserver")
            .arg("--stdio")
            .current_dir(repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

        self.create_client(SupportedLSPs::Python, process).await?;
        self.initialize_client(SupportedLSPs::Python, python_path.to_string())
            .await
    }

    async fn start_typescript_lsp(
        &mut self,
        repo_path: &str,
    ) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        warn!(
            "TypeScript LSP start requested but not implemented for repo: {}",
            repo_path
        );
        let _typescript_path = self.find_typescript_root(repo_path).await;
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "TypeScript LSP not implemented",
        )))
    }

    async fn start_rust_lsp(
        &mut self,
        repo_path: &str,
    ) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        warn!(
            "Rust LSP start requested but not implemented for repo: {}",
            repo_path
        );
        let _rust_path = self.find_rust_root(repo_path).await;
        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "Rust LSP not implemented",
        )))
    }

    async fn find_python_root(&mut self, repo_path: &str) -> String {
        repo_path.to_string()
    }

    async fn find_typescript_root(&mut self, repo_path: &str) -> String {
        self.find_tsconfig_files(repo_path)
            .first()
            .and_then(|path| path.parent())
            .map(|path| path.to_string_lossy().into_owned())
            .unwrap_or_else(|| repo_path.to_string())
    }

    fn find_tsconfig_files(&self, dir: &str) -> Vec<PathBuf> {
        let mut result = Vec::new();
        if let Ok(entries) = read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    result.extend(self.find_tsconfig_files(path.to_str().unwrap()));
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

    pub fn get_client(&self, lsp_type: SupportedLSPs) -> Option<Arc<Mutex<LspClient>>> {
        self.clients.get(&lsp_type).cloned()
    }

    fn detect_language(
        &self,
        file_path: &str,
    ) -> Result<SupportedLSPs, Box<dyn std::error::Error>> {
        // Open the file
        let mut file = File::open(file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        Ok(self.detect_language_from_content(&content))
    }

    fn detect_language_from_content(&self, _content: &str) -> SupportedLSPs {
        // For now, always return Python
        SupportedLSPs::Python
    }
}
