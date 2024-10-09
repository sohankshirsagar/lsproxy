use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::{Command, Child};
use crate::lsp_client::LspClient;
use log::{info, error};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::io::AsyncRead;

pub struct LspManager {
    clients: HashMap<PathBuf, (LspClient, Child)>,
}

impl LspManager {
    pub fn new() -> Self {
        LspManager {
            clients: HashMap::new(),
        }
    }

    pub async fn start_lsp_for_repo(&mut self, repo_path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if !self.clients.contains_key(&repo_path) {
            info!("Starting pyright for repo: {:?}", repo_path);
            let mut command = Command::new("pyright-langserver");
            command
                .arg("--stdio")
                .current_dir(&repo_path)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            let mut child = command.spawn()?;

            let stdin = child.stdin.take().expect("Failed to capture stdin");
            let stdout = child.stdout.take().expect("Failed to capture stdout");
            let stderr = child.stderr.take().expect("Failed to capture stderr");

            // Create LspClient
            let client = LspClient::new(stdin, stdout);

            // Log stderr
            self.log_output(BufReader::new(stderr));

            info!("Initializing LSP client for repo: {:?}", repo_path);
            match client.initialize(&repo_path.to_string_lossy()).await {
                Ok(_) => {
                    info!("LSP client initialized successfully for repo: {:?}", repo_path);
                    self.clients.insert(repo_path, (client, child));
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

    fn log_output(&self, stderr: impl AsyncRead + Unpin + Send + 'static) {
        let stderr = BufReader::new(stderr);
        tokio::spawn(async move {
            let mut reader = BufReader::new(stderr).lines();
            while let Some(line) = reader.next_line().await.expect("Failed to read line") {
                error!("pyright stderr: {}", line);
            }
        });
    }

    pub fn get_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Option<&mut LspClient> {
        self.clients.get_mut(repo_path).map(|(client, _)| client)
    }

    pub async fn stop_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        if let Some((client, mut child)) = self.clients.remove(repo_path) {
            client.shutdown().await?;
            child.kill().await?;
        }
        Ok(())
    }

    pub fn list_active_lsp_servers(&self) -> Vec<PathBuf> {
        self.clients.keys().cloned().collect()
    }
}
