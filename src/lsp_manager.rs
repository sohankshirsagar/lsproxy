use std::collections::HashMap;
use std::path::PathBuf;
use tokio::process::Child;
use tokio::process::Command;

pub struct LspManager {
    processes: HashMap<PathBuf, Child>,
}

impl LspManager {
    pub fn new() -> Self {
        LspManager {
            processes: HashMap::new(),
        }
    }

    pub async fn start_lsp_for_repo(&mut self, repo_path: PathBuf) -> Result<(), std::io::Error> {
        if !self.processes.contains_key(&repo_path) {
            let child = Command::new("pylsp")
                .arg("--tcp")
                .arg("--host")
                .arg("127.0.0.1")
                .arg("--port")
                .arg("0")  // Let the OS assign a port
                .current_dir(&repo_path)
                .spawn()?;

            self.processes.insert(repo_path, child);
        }
        Ok(())
    }

    pub fn get_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Option<&mut Child> {
        self.processes.get_mut(repo_path)
    }

    pub async fn stop_lsp_for_repo(&mut self, repo_path: &PathBuf) -> Result<(), std::io::Error> {
        if let Some(child) = self.processes.remove(repo_path) {
            child.kill().await?;
        }
        Ok(())
    }
}
