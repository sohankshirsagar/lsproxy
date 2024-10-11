use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::process::Stdio;
use log::{error, info, warn};
use tokio::sync::Mutex;
use tokio::process::Command;
use crate::lsp_client::LspClient;
use crate::types::{RepoKey, SupportedLSPs};
use std::fs::File;
use std::io::Read;
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, InitializeResult, Location};
use crate::symbol_finder::python_symbol_finder;

pub struct LspManager {
    clients: HashMap<(RepoKey, SupportedLSPs), Arc<Mutex<LspClient>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub async fn start_lsps(&mut self, key: RepoKey, repo_path: String, lsps: &[SupportedLSPs]) -> Result<(), String> {
        for &lsp in lsps {
            let result = match lsp {
                SupportedLSPs::Python => self.start_python_lsp(&key, &repo_path).await,
                SupportedLSPs::TypeScript => self.start_typescript_lsp(&key, &repo_path).await,
                SupportedLSPs::Rust => self.start_rust_lsp(&key, &repo_path).await,
            };
            if let Err(e) = result {
                error!("Failed to start {:?} LSP: {}", lsp, e);
                return Err(format!("Failed to start {:?} LSP: {}", lsp, e));
            }
        }
        Ok(())
    }

    async fn create_client(&mut self, key: RepoKey, lsp_type: SupportedLSPs, process: tokio::process::Child) -> Result<(), String> {
        // Await the async function LspClient::new
        let client = LspClient::new(process)
            .await
            .map_err(|e| format!("Failed to create LSP client: {}", e))?;
        let client = Arc::new(Mutex::new(client));
        self.clients.insert((key.clone(), lsp_type), client.clone());
        Ok(())
    }

    async fn initialize_client(&mut self, key: RepoKey, lsp_type: SupportedLSPs, repo_path: String) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        let client = self.get_client(&key, lsp_type)
            .ok_or("LSP client not found")?;

        // Initialize the client
        let mut locked_client = client.lock().await;
        locked_client.initialize(Some(repo_path.clone())).await
    }

    pub async fn get_symbols(&self, key: &RepoKey, file_path: &str) -> Result<DocumentSymbolResponse, Box<dyn std::error::Error>> {

        // Detect the language
        let lsp_type = self.detect_language(&file_path)?;

        let client = self.get_client(key, lsp_type)
            .ok_or("LSP client not found")?;

        // Call get_symbols on the LSP client
        let mut locked_client = client.lock().await;
        locked_client.get_symbols(file_path).await
    }

    pub async fn get_definition(&self, key: &RepoKey, file_path: &str, symbol_name: &str) -> Result<Vec<GotoDefinitionResponse>, Box<dyn std::error::Error>> {
        let mut unique_definitions = HashSet::new();
        let lsp_type = self.detect_language(file_path)?;
        
        if let Some(client) = self.get_client(key, lsp_type) {
            let occurrences = python_symbol_finder::find_symbol_occurrences(file_path, symbol_name)?;
            
            if occurrences.is_empty() {
                info!("No occurrences of symbol '{}' found in file '{}'", symbol_name, file_path);
                return Ok(vec![]);
            }
            
            info!("Found {} occurrences of symbol '{}' in file '{}'", occurrences.len(), symbol_name, file_path);
            
            let mut locked_client = client.lock().await;
            
            for occurrence in occurrences {
                match locked_client.get_definition(
                    file_path,
                    occurrence.start_line as u32 - 1,  // LSP uses 0-based line numbers
                    occurrence.start_column as u32 - 1 // LSP uses 0-based column numbers
                ).await {
                    Ok(definition) => {
                        info!("Found definition for symbol '{}' at line {}, column {}", 
                              symbol_name, occurrence.start_line, occurrence.start_column);
                        // Insert the definition into the HashSet
                        unique_definitions.insert(definition);
                    },
                    Err(e) => {
                        warn!("Failed to get definition for symbol '{}' at line {}, column {}: {}", 
                              symbol_name, occurrence.start_line, occurrence.start_column, e);
                    }
                }
            }
        } else {
            warn!("No LSP client found for file type {:?}", lsp_type);
        }
        
        let unique_definitions_vec: Vec<GotoDefinitionResponse> = unique_definitions.into_iter().collect();
        
        if unique_definitions_vec.is_empty() {
            info!("No unique definitions found for symbol '{}'", symbol_name);
        } else {
            info!("Found {} unique definition(s) for symbol '{}'", unique_definitions_vec.len(), symbol_name);
        }
        
        Ok(unique_definitions_vec)
    }

    async fn start_python_lsp(&mut self, key: &RepoKey, repo_path: &str) -> Result<InitializeResult, Box<dyn std::error::Error>> {
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

        self.create_client(key.clone(), SupportedLSPs::Python, process).await?;
        self.initialize_client(key.clone(), SupportedLSPs::Python, python_path.to_string()).await
    }

    async fn start_typescript_lsp(&mut self, _key: &RepoKey, repo_path: &str) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        warn!("TypeScript LSP start requested but not implemented for repo: {}", repo_path);
        let _typescript_path = self.find_typescript_root(repo_path).await;
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Unsupported, "TypeScript LSP not implemented")))
    }

    async fn start_rust_lsp(&mut self, _key: &RepoKey, repo_path: &str) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        warn!("Rust LSP start requested but not implemented for repo: {}", repo_path);
        let _rust_path = self.find_rust_root(repo_path).await;
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Unsupported, "Rust LSP not implemented")))
    }

    async fn find_python_root(&mut self, repo_path: &str) -> String {
        repo_path.to_string()
    }

    async fn find_typescript_root(&mut self, repo_path: &str) -> String {
        //TODO Actually find and verify
        repo_path.to_string()
    }

    async fn find_rust_root(&mut self, repo_path: &str) -> String{
        //TODO Actually find and verify
        repo_path.to_string()
    }

    pub fn get_client(&self, key: &RepoKey, lsp_type: SupportedLSPs) -> Option<Arc<Mutex<LspClient>>> {
        self.clients.get(&(key.clone(), lsp_type)).cloned()
    }

    fn detect_language(&self, file_path: &str) -> Result<SupportedLSPs, Box<dyn std::error::Error>> {
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
