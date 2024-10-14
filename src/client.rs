use crate::json_rpc::{JsonRpc, JsonRpcMessage};
use crate::process::Process;
use crate::protocol::LspProtocol;
use log::{debug, error, warn};
use lsp_types::{DocumentSymbolResponse, GotoDefinitionResponse, InitializeResult};
use serde::Serialize;
use std::error::Error;
use tokio::time::{timeout, Duration};

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
        let params = LspProtocol::initialize_params(root_path);
        let request = self
            .json_rpc
            .create_request("initialize", serde_json::to_value(params)?);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

        let response = self.receive_response().await?;
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

        let response = self.receive_response().await?;
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
        let params = LspProtocol::did_open_params(item);
        let notification = self
            .json_rpc
            .create_notification("textDocument/didOpen", params);
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
        let params = LspProtocol::goto_definition_params(file_path, line, character);
        let request = self
            .json_rpc
            .create_request("textDocument/definition", params);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

        let response = self.receive_response().await?;
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
        let params = LspProtocol::document_symbol_params(file_path);
        let request = self
            .json_rpc
            .create_request("textDocument/documentSymbol", params);
        let message = format!("Content-Length: {}\r\n\r\n{}", request.len(), request);
        self.process.send(&message).await?;

        let response = self.receive_response().await?;
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

    async fn receive_response(&mut self) -> Result<JsonRpcMessage, Box<dyn Error + Send + Sync>> {
        debug!("Awaiting response from LSP server");
        let response_str = timeout(Duration::from_secs(5), self.process.receive()).await??;
        debug!("Received raw response: {}", response_str.trim());

        // Parse headers
        let mut headers = response_str.lines();
        let content_length = if let Some(line) = headers.next() {
            if line.starts_with("Content-Length:") {
                line.trim_start_matches("Content-Length: ")
                    .trim()
                    .parse::<usize>()?
            } else {
                return Err(format!("Invalid header in response: {}", line).into());
            }
        } else {
            return Err("Empty response".into());
        };

        // Read the JSON body
        let mut body = vec![0; content_length];
        let body_str = timeout(Duration::from_secs(5), self.process.receive()).await??;
        body[..body_str.len()].copy_from_slice(body_str.as_bytes());

        let message = self.json_rpc.parse_message(&String::from_utf8(body)?)?;
        debug!("Parsed JSON-RPC message: {:?}", message);
        Ok(message)
    }
}
