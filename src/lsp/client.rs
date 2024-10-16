use crate::lsp::json_rpc::{JsonRpc, JsonRpcMessage};
use crate::lsp::process::Process;
use crate::lsp::{JsonRpcHandler, ProcessHandler};
use crate::utils::file_utils::search_directories;
use async_trait::async_trait;
use log::{debug, error, warn};
use lsp_types::{
    ClientCapabilities, DidOpenTextDocumentParams, DocumentSymbolClientCapabilities,
    DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams, GotoDefinitionResponse,
    InitializeParams, InitializeResult, Location, PartialResultParams, Position, ReferenceContext,
    ReferenceParams, TextDocumentClientCapabilities, TextDocumentIdentifier,
    TextDocumentPositionParams, Url, WorkDoneProgressParams, WorkspaceFolder,
    WorkspaceSymbolParams, WorkspaceSymbolResponse,
};
use std::error::Error;
use std::path::Path;

pub const DEFAULT_EXCLUDE_PATTERNS: &[&str] = &[
    "**/node_modules",
    "**/__pycache__",
    "**/.*",
    "**/dist",
    "**/target",
    "**/build",
    ".git",
];

#[async_trait]
pub trait LspClient: Send {
    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);

        let workspace_folders = self.find_workspace_folders(root_path.clone()).await?;
        debug!("Found workspace folders: {:?}", workspace_folders);
        let mut capabilities = ClientCapabilities::default();
        capabilities.text_document = Some(TextDocumentClientCapabilities {
            document_symbol: Some(DocumentSymbolClientCapabilities {
                hierarchical_document_symbol_support: Some(true),
                ..Default::default()
            }),
            ..Default::default()
        });

        capabilities.experimental = Some(
            serde_json::json!({
                "serverStatusNotification": true
            })
        );

        let params = InitializeParams {
            capabilities: capabilities,
            workspace_folders: Some(workspace_folders.clone()),
            root_uri: Some(workspace_folders[0].uri.clone()),
            ..Default::default()
        };
        let request = self
            .get_json_rpc()
            .create_request("initialize", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;
        let response = self.receive_response().await?.expect("No response");
        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)?;
            debug!("Initialization successful: {:?}", init_result);
            self.send_initialized().await?;
            Ok(init_result)
        } else if let Some(error) = response.error {
            error!("Initialization error: {:?}", error);
            Err(Box::new(error) as Box<dyn Error + Send + Sync>)
        } else {
            Err("Unexpected initialize response".into())
        }
    }

    async fn send_lsp_request(
        &mut self,
        method: &str,
        params: Option<serde_json::Value>,
    ) -> Result<serde_json::Value, Box<dyn Error + Send + Sync>> {
        debug!("Sending LSP request: {}", method);
        let request = self
            .get_json_rpc()
            .create_request(method, serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self.receive_response().await?.unwrap();

        if let Some(result) = response.result {
            debug!("Received response for {}", method);
            Ok(result)
        } else if let Some(error) = response.error {
            error!("Error in {} request: {:?}", method, error);
            Err(error.into())
        } else {
            warn!("No response for {} request", method);
            Ok(serde_json::Value::Null)
        }
    }

    async fn send_initialized(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending 'initialized' notification");
        let notification = self
            .get_json_rpc()
            .create_notification("initialized", serde_json::json!({}));
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            notification.len(),
            notification
        );
        self.get_process().send(&message).await
    }

    async fn text_document_did_open(
        &mut self,
        item: lsp_types::TextDocumentItem,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending 'didOpen' notification for document: {}", item.uri);
        let params = DidOpenTextDocumentParams {
            text_document: item,
        };
        let notification = self
            .get_json_rpc()
            .create_notification("textDocument/didOpen", serde_json::to_value(params)?);
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            notification.len(),
            notification
        );
        self.get_process().send(&message).await
    }

    async fn text_document_definition(
        &mut self,
        file_path: &str,
        position: Position,
    ) -> Result<GotoDefinitionResponse, Box<dyn Error + Send + Sync>> {
        debug!(
            "Requesting goto definition for {}, line {}, character {}",
            file_path, position.line, position.character
        );
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).unwrap(),
                },
                position: position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let request = self
            .get_json_rpc()
            .create_request("textDocument/definition", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self.receive_response().await?.expect("No response");
        if let Some(result) = response.result {
            let goto_resp: GotoDefinitionResponse = serde_json::from_value(result)?;
            debug!("Received goto definition response");
            Ok(goto_resp)
        } else if let Some(error) = response.error {
            error!("Goto definition error: {:?}", error);
            Err(error.into())
        } else {
            Err("Unexpected goto definition response".into())
        }
    }

    async fn workspace_symbols(
        &mut self,
        query: &str,
    ) -> Result<WorkspaceSymbolResponse, Box<dyn Error + Send + Sync>> {
        debug!("Requesting workspace symbols with query: {}", query);
        let params = WorkspaceSymbolParams {
            query: query.to_string(),
            ..Default::default()
        };
        let request = self
            .get_json_rpc()
            .create_request("workspace/symbol", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self
            .receive_response()
            .await?
            .ok_or("No response received")?;
        if let Some(result) = response.result {
            let symbols: WorkspaceSymbolResponse = serde_json::from_value(result)?;
            Ok(symbols)
        } else if let Some(error) = response.error {
            error!("Workspace symbols error: {:?}", error);
            Err(error.into())
        } else {
            Err("Unexpected workspace symbols response".into())
        }
    }

    async fn text_document_symbols(
        &mut self,
        file_path: &str,
    ) -> Result<DocumentSymbolResponse, Box<dyn Error + Send + Sync>> {
        debug!("Requesting document symbols for {}", file_path);
        let params = DocumentSymbolParams {
            text_document: TextDocumentIdentifier {
                uri: Url::from_file_path(file_path).unwrap(),
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let request = self
            .get_json_rpc()
            .create_request("textDocument/documentSymbol", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self.receive_response().await.unwrap().expect("No response");
        if let Some(result) = response.result {
            let symbols: DocumentSymbolResponse = serde_json::from_value(result)?;
            debug!("Received document symbols response");
            Ok(symbols)
        } else if let Some(error) = response.error {
            error!("Document symbols error: {:?}", error);
            Err(error.into())
        } else {
            Err("Unexpected document symbols response".into())
        }
    }

    async fn text_document_reference(
        &mut self,
        file_path: &str,
        position: Position,
        include_declaration: bool,
    ) -> Result<Vec<Location>, Box<dyn Error + Send + Sync>> {
        let params = ReferenceParams {
            text_document_position: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).map_err(|_| "Invalid file path")?,
                },
                position,
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
            context: ReferenceContext {
                include_declaration,
            },
        };

        let request = self
            .get_json_rpc()
            .create_request("textDocument/references", serde_json::to_value(params)?);

        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self
            .receive_response()
            .await?
            .ok_or("No response received")?;
        if let Some(result) = response.result {
            let references: Vec<Location> = serde_json::from_value(result)?;
            debug!("Received references response");
            Ok(references)
        } else if let Some(error) = response.error {
            error!("References error: {:?}", error);
            Err(error.into())
        } else {
            Err("Unexpected references response".into())
        }
    }

    async fn receive_response(
        &mut self,
    ) -> Result<Option<JsonRpcMessage>, Box<dyn Error + Send + Sync>> {
        debug!("Awaiting response from LSP server");
        // TODO this could be an inf loop, though timeout in receive will break it
        loop {
            let raw_response = self.get_process().receive().await?;
            let message = self.get_json_rpc().parse_message(&raw_response)?;
            debug!("Received response: {:?}", message);

            if let Some(msg_type) = &message.method {
                if msg_type == "window/logMessage" {
                    debug!("Captured log message, continuing to next message");
                    continue;
                }
            }

            if message.id.is_some() {
                return Ok(Some(message));
            }
        }
    }

    fn get_process(&mut self) -> &mut ProcessHandler;

    fn get_json_rpc(&mut self) -> &mut JsonRpcHandler;

    fn get_root_files(&mut self) -> Vec<String> {
        vec![".git".to_string()]
    }

    fn get_exclude_patterns(&mut self) -> Vec<String> {
        DEFAULT_EXCLUDE_PATTERNS
            .iter()
            .map(|&s| s.to_owned())
            .collect()
    }

    /// Sets up the workspace for the language server.
    ///
    /// Some language servers require specific commands to be run before
    /// workspace-wide features are available. For example:
    /// - TypeScript Language Server needs an explicit didOpen notification for each file
    /// - Rust Analyzer needs a reloadWorkspace command
    ///
    /// # Arguments
    ///
    /// * `root_path` - The root path of the workspace
    ///
    /// # Returns
    ///
    /// A Result containing () if successful, or a boxed Error if an error occurred
    #[allow(unused)]
    async fn setup_workspace(
        &mut self,
        root_path: &str,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        Ok(())
    }

    async fn find_workspace_folders(
        &mut self,
        root_path: String,
    ) -> Result<Vec<WorkspaceFolder>, Box<dyn Error + Send + Sync>> {
        let mut workspace_folders: Vec<WorkspaceFolder> = Vec::new();
        let include_patterns = self
            .get_root_files()
            .into_iter()
            .map(|f| format!("**/{f}"))
            .collect();
        debug!("include_patterns: {:?}", include_patterns);
        let exclude_patterns = self.get_exclude_patterns();

        match search_directories(&Path::new(&root_path), include_patterns, exclude_patterns) {
            Ok(dirs) => {
                for dir in dirs {
                    let folder_path = Path::new(&root_path).join(&dir);
                    if let Ok(uri) = Url::from_file_path(&folder_path) {
                        // remore folders that are parents of this one, because we prefer more specific paths
                        // this is pretty inefficient but moving on
                        workspace_folders.retain(|folder: &WorkspaceFolder| {
                            !uri.to_file_path()
                                .unwrap()
                                .starts_with(folder.uri.to_file_path().unwrap())
                        });

                        workspace_folders.push(WorkspaceFolder {
                            uri,
                            name: folder_path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("")
                                .to_string(),
                        });
                    }
                }
            }
            Err(e) => return Err(Box::new(e)),
        }

        Ok(workspace_folders.into_iter().collect())
    }
}
