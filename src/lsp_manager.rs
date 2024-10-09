use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use crate::lsp_client::LspClient;

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

            Command::new("pylsp")
                .arg("--tcp")
                .arg("--host")
                .arg("127.0.0.1")
                .arg("--port")
                .arg(port.to_string())
                .current_dir(&repo_path)
                .spawn()?;

            // Wait a bit for the server to start
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            let client = LspClient::new(port).await?;
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
