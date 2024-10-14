use std::{error::Error, process::Stdio};

use async_trait::async_trait;
use log::{debug, warn};
use lsp_types::WorkspaceFolder;
use tokio::process::Command;

use crate::{
    lsp::{JsonRpcHandler, LspClient, ProcessHandler},
    utils::get_files_for_workspace_typescript,
};

pub struct TypeScriptClient {
    process: ProcessHandler,
    json_rpc: JsonRpcHandler,
}

#[async_trait]
impl LspClient for TypeScriptClient {
    fn get_process(&mut self) -> &mut ProcessHandler {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler {
        &mut self.json_rpc
    }

    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        /* 
        The server needs to know about all the files in the workspace to provide features like go to definition.
        This is a limitation of the TypeScript language server.
         */
        debug!("Setting up workspace for TypeScript client");
        let text_document_items = get_files_for_workspace_typescript(root_path).await.unwrap();
        for item in text_document_items {
            debug!("Sent 'didOpen' for file: {}", item.uri.to_string());
            self.text_document_did_open(item).await?;
        }
        debug!("Workspace setup completed for TypeScript client");
        Ok(())
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        warn!("TypeScriptClient does not support finding workspace folders");
        Ok(vec![])
    }
}

impl TypeScriptClient {
    pub async fn new(root_path: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let process = Command::new("typescript-language-server")
            .arg("--stdio")
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

        Ok(Self {
            process: process_handler,
            json_rpc: json_rpc_handler,
        })
    }
}
