use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use serde_json::Value;
use lsp_types::{
    InitializeParams, InitializeResult, ClientCapabilities, TextDocumentClientCapabilities,
    WorkspaceClientCapabilities, WorkspaceFolder, DocumentSymbolParams, DocumentSymbolResponse,
    GotoDefinitionParams, GotoDefinitionResponse, TextDocumentPositionParams, Position,
    Url,
};
use log::{error, debug, warn};
use tokio::time::Duration;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcMessage {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: Option<String>,
    params: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct JsonRpcError {
    code: i32,
    message: String,
    data: Option<serde_json::Value>,
}

pub struct LspClient {
    curr_id: u32,
    child: tokio::process::Child,
    stdin: ChildStdin,
    stdout: tokio::io::BufReader<ChildStdout>,
}

impl LspClient {
    pub async fn new(mut child: tokio::process::Child) -> Result<Self, Box<dyn std::error::Error>> {
        debug!("Creating new LspClient");
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        debug!("Opened stdin");
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        debug!("Opened stdout");
        let stderr = child.stderr.take().ok_or("Failed to open stderr")?;
        debug!("Opened stderr");

        let stdout = tokio::io::BufReader::new(stdout);

        // Pipe stderr
        let stderr_reader = tokio::io::BufReader::new(stderr);
        tokio::spawn(async move {
            let mut lines = stderr_reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                eprintln!("LSP stderr: {}", line);
            }
        });

        // Check if the child process is still running
        match child.try_wait() {
            Ok(Some(status)) => {
                error!("LSP server process exited immediately with status: {:?}", status);
                return Err("LSP server process exited immediately".into());
            }
            Ok(None) => debug!("LSP server process is still running"),
            Err(e) => warn!("Error checking LSP server process status: {}", e),
        }

        Ok(LspClient {
            curr_id: 1,
            child,
            stdin,
            stdout,
        })
    }

    pub async fn initialize(&mut self, repo_path: Option<String>) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        debug!("Initializing LSP client with repo path: {:?}", repo_path);
        let capabilities = ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities::default()),
            workspace: Some(WorkspaceClientCapabilities::default()),
            ..Default::default()
        };

        let mut params = InitializeParams {
            capabilities,
            ..Default::default()
        };

        if let Some(path) = repo_path {
            params.workspace_folders = Some(vec![WorkspaceFolder {
                uri: lsp_types::Url::from_file_path(path).map_err(|_| "Invalid repo path")?,
                name: "workspace".to_string(),
            }]);
        }

        let result: InitializeResult = self.send_lsp_request("initialize", params).await?;

        debug!("Sending initialized notification");
        let notification = self.create_notification("initialized", serde_json::json!({}));
        if let Err(e) = self.send_notification(&notification).await {
            error!("Failed to send initialized notification: {}", e);
            return Err(e);
        }

        debug!("LSP client initialization completed successfully");
        Ok(result)
    }

    pub async fn get_symbols(&mut self, file_path: &str) -> Result<DocumentSymbolResponse, Box<dyn std::error::Error>> {
        debug!("Getting symbols for file: {}", file_path);

        let uri = Url::from_file_path(Path::new(file_path))
            .map_err(|_| format!("Invalid file path: {}", file_path))?;

        let params = DocumentSymbolParams {
            text_document: lsp_types::TextDocumentIdentifier { uri },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        self.send_lsp_request("textDocument/documentSymbol", params).await
    }

    pub async fn get_definition(&mut self, file_path: &str, line: u32, character: u32) -> Result<GotoDefinitionResponse, Box<dyn std::error::Error>> {
        debug!("Getting definition for file: {}, line: {}, character: {}", file_path, line, character);

        let uri = Url::from_file_path(Path::new(file_path))
            .map_err(|_| format!("Invalid file path: {}", file_path))?;

        let params = GotoDefinitionParams {
            text_document_position_params: TextDocumentPositionParams {
                text_document: lsp_types::TextDocumentIdentifier { uri },
                position: Position { line, character },
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        self.send_lsp_request("textDocument/definition", params).await
    }

    async fn send_lsp_request<T, U>(&mut self, method: &str, params: T) -> Result<U, Box<dyn std::error::Error>>
    where
        T: serde::Serialize,
        U: serde::de::DeserializeOwned,
    {
        let request = self.create_jsonrpc_request(method, serde_json::to_value(params)?);
        debug!("Sending {} request: {}", method, request);
        self.send_jsonrpc_request(&request).await?;

        loop {
            let response = self.read_response().await?;
            debug!("Received response: {:?}", response);

            if let Some(msg_type) = &response.method {
                if msg_type == "window/logMessage" {
                    debug!("Captured log message, continuing to next message");
                    continue;
                }
            }

            if let Some(error) = response.error {
                error!("Error in {} response: {:?}", method, error);
                return Err(format!("Error in {} response: {}", method, error.message).into());
            }

            return match response.result {
                Some(result) => {
                    let parsed: U = serde_json::from_value(result)
                        .map_err(|e| format!("Failed to parse {} response: {}", method, e))?;
                    Ok(parsed)
                }
                None => {
                    error!("No result in {} response: {:?}", method, response);
                    Err(format!("No result in {} response", method).into())
                }
            };
        }
    }

    fn create_jsonrpc_request(&mut self, method: &str, params: Value) -> String {
        let id = self.curr_id;
        self.curr_id += 1;
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params
        }).to_string()
    }

    fn create_notification(&self, method: &str, params: Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        }).to_string()
    }

    async fn send_jsonrpc_request(&mut self, request: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content_length = request.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_length);
        
        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(request.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn send_notification(&mut self, notification: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content_length = notification.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_length);
        
        self.stdin.write_all(header.as_bytes()).await?;
        self.stdin.write_all(notification.as_bytes()).await?;
        self.stdin.flush().await?;

        Ok(())
    }

    async fn read_response(&mut self) -> Result<JsonRpcMessage, Box<dyn std::error::Error>> {
        debug!("Starting to read response from LSP server");
        let mut content_length: Option<usize> = None;
        let mut buffer = Vec::new();
        let mut timeout = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                result = self.stdout.read_until(b'\n', &mut buffer) => {
                    match result {
                        Ok(0) => {
                            debug!("Reached EOF");
                            return Err("Unexpected EOF".into());
                        }
                        Ok(n) => {
                            let line = String::from_utf8_lossy(&buffer[buffer.len() - n..]);
                            debug!("Read line ({} bytes): {}", n, line.trim());

                            if line.trim().is_empty() {
                                if let Some(length) = content_length {
                                    // Read the JSON content
                                    let mut content = vec![0; length];
                                    self.stdout.read_exact(&mut content).await?;
                                    debug!("Read JSON content: {}", String::from_utf8_lossy(&content));
                                    
                                    let response: JsonRpcMessage = serde_json::from_slice(&content)
                                        .map_err(|e| format!("Failed to parse JSON: {}. Content: {}", e, String::from_utf8_lossy(&content)))?;
                                    
                                    debug!("Successfully parsed JSON response");
                                    return Ok(response);
                                }
                            } else if line.starts_with("Content-Length: ") {
                                content_length = Some(line.trim_start_matches("Content-Length: ").trim().parse()?);
                                debug!("Content-Length found: {:?}", content_length);
                            } else {
                                // Log non-JSON-RPC messages
                                self.log_non_json_rpc("stdout", &line);
                            }
                        }
                        Err(e) => {
                            error!("Error reading from stdout: {}", e);
                            return Err(e.into());
                        }
                    }
                    buffer.clear();
                }
                _ = timeout.tick() => {
                    warn!("Timeout while reading response");
                    // Check if the child process is still running
                    match self.child.try_wait() {
                        Ok(Some(status)) => {
                            error!("LSP server process exited with status: {:?}", status);
                            return Err("LSP server process exited unexpectedly".into());
                        }
                        Ok(None) => warn!("LSP server process is still running, but not responding"),
                        Err(e) => warn!("Error checking LSP server process status: {}", e),
                    }
                    continue;
                }
            }
        }
    }

    fn log_non_json_rpc(&self, stream: &str, message: &str) {
        debug!("Non-JSON-RPC message from {}: {}", stream, message);
    }
}
