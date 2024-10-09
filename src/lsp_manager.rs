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

            info!("Starting pylsp for repo: {:?}", repo_path);
            let mut command = Command::new("pylsp");
            command
                .arg("--tcp")
                .arg("--host")
                .arg("127.0.0.1")
                .arg("--port")
                .arg(port.to_string())
                .current_dir(&repo_path)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn()?;

            // Capture stdout and stderr
            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stdout).lines();
                while let Some(line) = reader.next_line().await.expect("Failed to read line") {
                    debug!("pylsp stdout: {}", line);
                }
            });

            tokio::spawn(async move {
                let mut reader = tokio::io::BufReader::new(stderr).lines();
                while let Some(line) = reader.next_line().await.expect("Failed to read line") {
                    error!("pylsp stderr: {}", line);
                }
            });

            // Wait a bit for the server to start
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            info!("Attempting to connect to pylsp on port {}", port);
            let client = LspClient::new(port).await?;
            
            info!("Initializing LSP client for repo: {:?}", repo_path);
            client.initialize(&repo_path.to_string_lossy()).await?;
            
            info!("LSP client initialized successfully for repo: {:?}", repo_path);
            self.clients.insert(repo_path, client);
        }
        Ok(())
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
