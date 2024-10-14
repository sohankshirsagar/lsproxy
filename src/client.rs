use crate::json_rpc::{JsonRpc, JsonRpcMessage};
use crate::process::Process;
use log::{debug, error, warn};
use lsp_types::{
    DidOpenTextDocumentParams, DocumentSymbolParams, DocumentSymbolResponse, GotoDefinitionParams,
    GotoDefinitionResponse, InitializeParams, InitializeResult, PartialResultParams, Position,
    TextDocumentIdentifier, TextDocumentPositionParams, Url, WorkDoneProgressParams,
    WorkspaceFolder,
};
use serde::Serialize;
use std::error::Error;

pub struct LspClient<P: Process, J: JsonRpc> {
    process: P,
    json_rpc: J,
}

impl<P: Process, J: JsonRpc> LspClient<P, J> {
    pub fn new(process: P, json_rpc: J) -> Self {
        Self { process, json_rpc }
    }

    pub async fn initialize(
        &mut self,
        root_path: Option<String>,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>> {
        debug!("Initializing LSP client with root path: {:?}", root_path);
        let params = InitializeParams {
            capabilities: Default::default(),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(root_path.clone().unwrap()).unwrap(),
                name: root_path.clone().unwrap(),
            }]),
            root_uri: Some(Url::from_file_path(root_path.clone().unwrap()).unwrap()),
            ..Default::default()
        };
        let request = self
            .json_rpc
            .create_request("initialize", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

        let response = self.receive_response().await.unwrap().expect("No response");
        if let Some(result) = response.result {
            let init_result: InitializeResult = serde_json::from_value(result)?;
            debug!("Initialization successful: {:?}", init_result);
            // Send initialized notification
            self.send_initialized().await?;
            Ok(init_result)
        } else if let Some(error) = response.error {
            error!("Initialization error: {:?}", error);
            Err(Box::new(error) as Box<dyn Error + Send + Sync>)
        } else {
            Err("Unexpected initialize response".into())
        }
    }

    pub async fn send_lsp_request<T: Serialize, R: serde::de::DeserializeOwned>(
        &mut self,
        method: &str,
        params: T,
    ) -> Result<R, Box<dyn Error + Send + Sync>> {
        debug!("Sending LSP request: {}", method);
        let request = self
            .json_rpc
            .create_request(method, serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

        let response = self.receive_response().await.unwrap().expect("No response");
        if let Some(result) = response.result {
            let result: R = serde_json::from_value(result)?;
            debug!("Received response for {}", method);
            Ok(result)
        } else if let Some(error) = response.error {
            error!("Error in {} request: {:?}", method, error);
            Err(error.into())
        } else {
            Err(format!("Unexpected response for {} request", method).into())
        }
    }

    async fn send_initialized(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending 'initialized' notification");
        let notification = self
            .json_rpc
            .create_notification("initialized", serde_json::json!({}));
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            notification.len(),
            notification
        );
        self.process.send(&message).await
    }

    pub async fn text_document_did_open(
        &mut self,
        item: lsp_types::TextDocumentItem,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending 'didOpen' notification for document: {}", item.uri);
        let params = DidOpenTextDocumentParams {
            text_document: item,
        };
        let notification = self
            .json_rpc
            .create_notification("textDocument/didOpen", serde_json::to_value(params)?);
        let message = format!(
            "Content-Length: {}\r\n\r\n{}",
            notification.len(),
            notification
        );
        self.process.send(&message).await
    }

    pub async fn goto_definition(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<GotoDefinitionResponse, Box<dyn Error + Send + Sync>> {
        debug!(
            "Requesting goto definition for {}, line {}, character {}",
            file_path, line, character
        );
        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: TextDocumentIdentifier {
                    uri: Url::from_file_path(file_path).unwrap(),
                },
                position: Position {
                    line: line,
                    character: character,
                },
            },
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        };
        let request = self
            .json_rpc
            .create_request("textDocument/definition", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

        let response = self.receive_response().await.unwrap().expect("No response");
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

    pub async fn text_document_symbols(
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
            .json_rpc
            .create_request("textDocument/documentSymbol", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

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

    async fn receive_response(
        &mut self,
    ) -> Result<Option<JsonRpcMessage>, Box<dyn Error + Send + Sync>> {
        debug!("Awaiting response from LSP server");
        let raw_response = self.process.receive().await?;
        let message = self.json_rpc.parse_message(&raw_response)?;
        if message.id.is_some() {
            return Ok(Some(message));
        }
        Ok(None)
    }
}
