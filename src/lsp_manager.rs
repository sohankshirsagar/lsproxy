use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use crate::lsp_client::LspClient;
use log::{info, error, debug};
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;

pub struct LspManager {
    clients: HashMap<PathBuf, LspClient>,
    next_port: u16,
}

impl LspManager {
    pub fn new() -> Self {
        LspManager {
            clients: HashMap::new(),
            next_port: 2760,
        }
    }

    pub async fn start_lsp_for_repo(&mut self, repo_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if !self.clients.contains_key(&repo_path) {
            let port = self.next_port;
            self.next_port += 1;

            info!("Starting pyright for repo: {:?}", repo_path);
            let mut command = Command::new("pyright-langserver");
            command
                .arg("--stdio")
                .current_dir(&repo_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn()?;

            // Capture stdout and stderr
            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                while let Some(line) = reader.next_line().await.expect("Failed to read line") {
                    debug!("pyright stdout: {}", line);
                }
            });

            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stderr).lines();
                while let Some(line) = reader.next_line().await.expect("Failed to read line") {
                    error!("pyright stderr: {}", line);
                }
            });

            // Increase wait time for the server to start
            info!("Waiting for pyright server to start...");
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;

            info!("Attempting to connect to pyright");
            let client = match LspClient::new(port).await {
                Ok(c) => c,
                Err(e) => {
                    error!("Failed to create LSP client: {}", e);
                    return Err(Box::new(e));
                }
            };
            
            info!("Initializing LSP client for repo: {:?}", repo_path);
            match client.initialize(&repo_path.to_string_lossy()).await {
                Ok(_) => {
                    info!("LSP client initialized successfully for repo: {:?}", repo_path);
                    self.clients.insert(repo_path, client);
                    Ok(())
                },
                Err(e) => {
                    error!("Failed to initialize LSP client: {}", e);
                    Err(e)
                }
            }
        } else {
            info!("LSP client already exists for repo: {:?}", repo_path);
            Ok(())
        }
    }

    pub fn get_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Option<&mut LspClient> {
        self.clients.get_mut(repo_path)
    }

    pub async fn stop_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Result<(), std::io::Error> {
        if let Some(client) = self.clients.remove(repo_path) {
            client.shutdown().await?;
        }
        Ok(())
    }

    pub fn list_active_lsp_servers(&self) -> Vec<PathBuf> {
        self.clients.keys().cloned().collect()
    }
}
