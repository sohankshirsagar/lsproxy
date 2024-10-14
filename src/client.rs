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

pub trait LspClient<P: Process, J: JsonRpc> {
    async fn initialize(
        &mut self,
        root_path: String,
    ) -> Result<InitializeResult, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
        debug!("Initializing LSP client with root path: {:?}", root_path);
        let params = InitializeParams {
            capabilities: Default::default(),
            workspace_folders: Some(vec![WorkspaceFolder {
                uri: Url::from_file_path(root_path.clone()).unwrap(),
                name: root_path.clone(),
            }]),
            root_uri: Some(Url::from_file_path(root_path.clone()).unwrap()),
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

    async fn send_lsp_request<T: Serialize, R: serde::de::DeserializeOwned + Default>(
        &mut self,
        method: &str,
        params: T,
    ) -> Result<R, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
        debug!("Sending LSP request: {}", method);
        let request = self
            .get_json_rpc()
            .create_request(method, serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.get_process().send(&message).await?;

        let response = self.receive_response().await?.unwrap();

        if let Some(result) = response.result {
            let result: R = serde_json::from_value(result)?;
            debug!("Received response for {}", method);
            Ok(result)
        } else if let Some(error) = response.error {
            error!("Error in {} request: {:?}", method, error);
            Err(error.into())
        } else {
            warn!("No response for {} request", method);
            Ok(R::default())
        }
    }

    async fn send_initialized(&mut self) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
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
    ) -> Result<(), Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
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

    async fn goto_definition(
        &mut self,
        file_path: &str,
        line: u32,
        character: u32,
    ) -> Result<GotoDefinitionResponse, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
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

    async fn text_document_symbols(
        &mut self,
        file_path: &str,
    ) -> Result<DocumentSymbolResponse, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
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

    async fn receive_response(
        &mut self,
    ) -> Result<Option<JsonRpcMessage>, Box<dyn Error + Send + Sync>>
    where
        Self: Sized,
    {
        debug!("Awaiting response from LSP server");
        // todo this could be an inf loop, though timeout in receive will break it
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

    // Helper methods to access fields
    fn get_process(&mut self) -> &mut P
    where
        Self: Sized;
    fn get_json_rpc(&mut self) -> &mut J
    where
        Self: Sized;
}

pub struct GenericClient<P: Process, J: JsonRpc> {
    process: P,
    json_rpc: J,
}

impl<P: Process, J: JsonRpc> GenericClient<P, J> {
    pub fn new(process: P, json_rpc: J) -> Self {
        Self { process, json_rpc }
    }
}

// Implement the trait for LspClient
impl<P: Process, J: JsonRpc> LspClient<P, J> for GenericClient<P, J> {

    fn get_process(&mut self) -> &mut P {
        &mut self.process
    }

    fn get_json_rpc(&mut self) -> &mut J {
        &mut self.json_rpc
    }
}
