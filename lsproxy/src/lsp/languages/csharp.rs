use crate::{
    lsp::{JsonRpcHandler, LspClient, PendingRequests, ProcessHandler},
    utils::{file_utils::search_files, workspace_documents::{
        DidOpenConfiguration, WorkspaceDocumentsHandler, CSHARP_FILE_PATTERNS, CSHARP_ROOT_FILES, DEFAULT_EXCLUDE_PATTERNS
    }},
};
use async_trait::async_trait;
use lsp_types::InitializeParams;
use notify_debouncer_mini::DebouncedEvent;
use log::{info, warn};
use std::{error::Error, path::Path, process::Stdio};
use tokio::{process::Command, sync::broadcast::Receiver};
pub struct CSharpClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
    workspace_documents: WorkspaceDocumentsHandler,
    pending_requests: PendingRequests,
}
#[async_trait]
impl LspClient for CSharpClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }
    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }
    fn get_root_files(&mut self) -> Vec<String> {
        CSHARP_ROOT_FILES.iter().map(|&s| s.to_owned()).collect()
    }
    fn get_workspace_documents(&mut self) -> &mut WorkspaceDocumentsHandler {
        &mut self.workspace_documents
    }
    fn get_pending_requests(&mut self) -> &mut PendingRequests {
        &mut self.pending_requests
    }

    async fn get_initialize_params(
        &mut self,
        root_path: String,
    ) -> Result<InitializeParams, Box<dyn Error + Send + Sync>> {
        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;
        Ok(InitializeParams {
            capabilities: self.get_capabilities(),
            workspace_folders: Some(workspace_folders.clone()),
            root_uri: workspace_folders.first().map(|f| f.uri.clone()), // <--------- Not default behavior
            ..Default::default()
        })
    }
}
impl CSharpClient {
    pub async fn new(
        root_path: &str,
        watch_events_rx: Receiver<DebouncedEvent>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let debug_file = std::fs::File::create("/tmp/csharp.log")?;
        let mut cmd = Command::new("csharp-ls");
        cmd.current_dir(root_path);

        match search_files(
            Path::new(root_path), 
            vec![String::from("**/*.sln")], 
            DEFAULT_EXCLUDE_PATTERNS.iter().map(|s| s.to_string()).collect(), 
            true
        ) {
            Ok(solution_files) => {
                if solution_files.is_empty() {
                    info!("No solution files found, using root directory");
                } else {
                    if solution_files.len() > 1 {
                        warn!(
                            "Multiple solution files found. Using '{:?}'. Ignoring: {:?}", 
                            solution_files[0],
                            &solution_files[1..]
                        );
                    }
                    cmd.arg("--solution").arg(&solution_files[0]);
                }
            },
            Err(e) => {
                warn!("Failed to search for solution files: {}", e);
                info!("Continuing without solution file");
            }
        };

        let process = cmd
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(debug_file)
            .spawn()
            .map_err(|e| {
                eprintln!("Failed to start ruby-lsp process: {}", e);
                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
            })?;
//        let process = Command::new("OmniSharp")
//            .arg("--lsp")
//            .current_dir(root_path)
//            .stdin(Stdio::piped())
//            .stdout(Stdio::piped())
//            .stderr(debug_file)
//            .spawn()
//            .map_err(|e| {
//                eprintln!("Failed to start omnisharp process: {}", e);
//                Box::new(e) as Box<dyn std::error::Error + Send + Sync>
//            })?;
        let process_handler = ProcessHandler::new(process)
            .await
            .map_err(|e| format!("Failed to create ProcessHandler: {}", e))?;
        let json_rpc_handler = JsonRpcHandler::new();
        let workspace_documents = WorkspaceDocumentsHandler::new(
            Path::new(root_path),
            CSHARP_FILE_PATTERNS.iter().map(|&s| s.to_string()).collect(),
            DEFAULT_EXCLUDE_PATTERNS
                .iter()
                .map(|&s| s.to_string())
                .collect(),
            watch_events_rx,
            DidOpenConfiguration::Lazy,
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
