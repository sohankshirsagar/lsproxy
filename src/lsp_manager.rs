use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Command;
use crate::lsp_client::LspClient;

pub struct LspManager {
    clients: HashMap<PathBuf, LspClient>,
}

impl LspManager {
    pub fn new() -> Self {
        LspManager {
            clients: HashMap::new(),
        }
    }

    pub async fn start_lsp_for_repo(&mut self, repo_path: PathBuf) -> Result<(), std::io::Error> {
        if !self.clients.contains_key(&repo_path) {
            let port = 2760;
            let child = Command::new("pylsp")
                .arg("--tcp")
                .arg("--host")
                .arg("127.0.0.1")
                .arg("--port")
                .arg(port.to_string())
                .current_dir(&repo_path)
                .spawn()?;

            let client = LspClient::new(child, port);
            self.clients.insert(repo_path, client);
        }
        Ok(())
    }

    pub fn get_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Option<&mut LspClient> {
        self.clients.get_mut(repo_path)
    }

    pub async fn stop_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Result<(), std::io::Error> {
        if let Some(mut client) = self.clients.remove(repo_path) {
            client.kill().await?;
        }
        Ok(())
    }

    pub fn list_active_lsp_servers(&self) -> Vec<PathBuf> {
        self.clients.keys().cloned().collect()
    }
}
