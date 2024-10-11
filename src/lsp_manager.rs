use std::collections::HashMap;
use std::sync::Arc;
use std::process::Stdio;
use log::{error, info, warn};
use tokio::sync::Mutex;
use tokio::process::Command;
use crate::lsp_client::LspClient;
use crate::types::{RepoKey, SupportedLSPs};
use std::fs::File;
use std::io::Read;
use lsp_types::DocumentSymbolResponse;

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

    async fn start_python_lsp(&mut self, key: &RepoKey, repo_path: &str) -> Result<(), String> {
        let python_path = self.find_python_root(repo_path).await;
        // Spawn the LSP server using tokio's async process
        let process = match Command::new("pyright-langserver")
            .arg("--stdio")
            .current_dir(python_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() {
                Ok(child) => child,
                Err(e) => {
                    error!("Failed to start Pyright LSP for repo {}: {}", repo_path, e);
                    return Err(format!("Failed to start Pyright LSP: {}", e));
                }
            };

        self.create_and_initialize_client(key.clone(), SupportedLSPs::Python, process, repo_path.to_string()).await
    }

    async fn start_typescript_lsp(&mut self, _key: &RepoKey, repo_path: &str) -> Result<(), String> {
        warn!("TypeScript LSP start requested but not implemented for repo: {}", repo_path);
        let _typescript_path = self.find_typescript_root(repo_path);
        Err("TypeScript LSP not implemented".to_string())
    }

    async fn start_rust_lsp(&mut self, _key: &RepoKey, repo_path: &str) -> Result<(), String> {
        warn!("Rust LSP start requested but not implemented for repo: {}", repo_path);
        let _rust_path = self.find_rust_root(repo_path);
        Err("Rust LSP not implemented".to_string())
    }

    async fn create_and_initialize_client(&mut self, key: RepoKey, lsp_type: SupportedLSPs, process: tokio::process::Child, repo_path: String) -> Result<(), String> {
        // Await the async function LspClient::new
        let client = LspClient::new(process)
            .await
            .map_err(|e| format!("Failed to create LSP client: {}", e))?;
        let client = Arc::new(Mutex::new(client));
        self.clients.insert((key.clone(), lsp_type), client.clone());

        // Initialize the client
        let mut locked_client = client.lock().await;
        locked_client.initialize(Some(repo_path.clone()))
            .await
            .map_err(|e| format!("Failed to initialize LSP client: {}", e))?;

        info!("Started and initialized {:?} LSP for repo: {}", lsp_type, repo_path);
        Ok(())
    }

    async fn find_python_root(&mut self, repo_path: &str) -> String {
        //TODO Actually find and verify
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

    pub async fn get_symbols(&self, key: &RepoKey, file_path: &str) -> Result<DocumentSymbolResponse, Box<dyn std::error::Error>> {
        // Open the file
        let mut file = File::open(file_path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;

        // Detect the language
        let lsp_type = self.detect_language(&content);

        let client = self.get_client(key, lsp_type)
            .ok_or("LSP client not found")?;

        // Call get_symbols on the LSP client
        let mut locked_client = client.lock().await;
        locked_client.get_symbols(file_path).await
    }

    fn detect_language(&self, _content: &str) -> SupportedLSPs {
        // For now, always return Python
        SupportedLSPs::Python
    }
}
