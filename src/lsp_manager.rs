use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use log::{error, info};

mod lsp_client;
use lsp_client::LspClient;

pub struct LspManager {
    clients: HashMap<(String, String), Arc<Mutex<LspClient>>>,
}

impl LspManager {
    pub fn new() -> Self {
        Self {
            clients: HashMap::new(),
        }
    }

    pub fn start_lsp(&mut self, id: String, github_url: String, repo_path: String) -> Result<(), String> {
        let key = (id.clone(), github_url.clone());

        if self.clients.contains_key(&key) {
            return Err("LSP client already exists for this repository".to_string());
        }

        let process = match Command::new("pyright-langserver")
            .arg("--stdio")
            .current_dir(&repo_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn() {
                Ok(child) => child,
                Err(e) => {
                    error!("Failed to start Pyright LSP for repo {}: {}", github_url, e);
                    return Err(format!("Failed to start Pyright LSP: {}", e));
                }
            };

        let client = Arc::new(Mutex::new(LspClient::new(process)));
        self.clients.insert(key, client);

        info!("Started Pyright LSP for repo: {}", github_url);
        Ok(())
    }

    pub fn get_client(&self, id: &str, github_url: &str) -> Option<Arc<Mutex<LspClient>>> {
        self.clients.get(&(id.to_string(), github_url.to_string())).cloned()
    }
}
