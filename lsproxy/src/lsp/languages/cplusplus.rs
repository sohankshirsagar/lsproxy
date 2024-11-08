use std::path::Path;
use std::process::Stdio;

use async_trait::async_trait;
use notify_debouncer_mini::DebouncedEvent;
use tokio::{process::Command, sync::broadcast::Receiver};

use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::{
        file_utils::search_directory_for_string,
        workspace_documents::{
            WorkspaceDocumentsHandler, CPP_FILE_PATTERNS, CPP_ROOT_FILES, DEFAULT_EXCLUDE_PATTERNS,
        },
    },
};

pub struct ClangdClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}

#[async_trait]
impl LspClient for ClangdClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    fn get_root_files(&mut self) -> Vec<String> {
        CPP_ROOT_FILES.iter().map(|&s| s.to_owned()).collect()
    }

    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }

    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }
}

impl ClangdClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // Clangd requires a compile_commands.json to be present. On the one hand, this makes configuration easier,
        // on the other hand, if there's no compile_commands.json present we need to create one.
        if !Path::new(&format!("{}/compile_commands.json", root_path)).exists() {
            //if there's a makefile, run that.
            if Path::new(&format!("{}/makefile", root_path)).exists() {
                Command::new("compiledb")
                    .arg("-n")
                    .arg("make")
                    .current_dir(root_path)
                    .spawn()
                    .expect("Couldn't compiledb to generate compile_commands");
            } else {
                //if there's not a makefile, try to detect main and use that for compile_commands generation
                let rp = Path::new(root_path);
                let mut inc_pats = Vec::new();
                CPP_FILE_PATTERNS
                    .iter()
                    .for_each(|fp| inc_pats.push(fp.to_string()));
                match search_directory_for_string(rp, inc_pats, Vec::new(), "int main".to_string())
                {
                    Ok(files) => {
                        println!("Found files for clangd: {:?}, just using the first one for main compilation",files);
                        match files[0].file_stem() {
                            Some(stem) => {
                                println!("Using {:?}", stem.to_str());
                                Command::new("compiledb")
                                    .arg("-n")
                                    .arg("make")
                                    .arg(stem)
                                    .current_dir(root_path)
                                    .spawn()
                                    .expect("Couldn't compiledb to generate compile_commands");
                            }
                            None => {
                                println!("For some reason, the first file we found doesn't have a name. This will cause an error.")
                            }
                        }
                    }
                    Err(_) => {}
                }
            }
        }

        let process = Command::new("clangd")
            .current_dir(root_path)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;

        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();
        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            CPP_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
        );
        let pending_requests = PendingRequests::new();

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
            workspace_documents,
            pending_requests,
        })
    }
}
