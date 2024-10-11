use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use serde_json::Value;
use lsp_types::{InitializeParams, InitializeResult, ClientCapabilities, TextDocumentClientCapabilities, WorkspaceClientCapabilities, WorkspaceFolder};
use log::{error, debug, warn};
use tokio::time::Duration;
use serde::{Deserialize, Serialize};

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

        let request = self.create_request("initialize", serde_json::to_value(params)?);
        debug!("Sending initialize request: {}", request);
        self.send_request(&request).await?;

        let final_response;
        loop {
            let response = match self.read_response().await {
                Ok(resp) => resp,
                Err(e) => {
                    error!("Failed to read response: {}", e);
                    return Err(e.into());
                }
            };
            debug!("Received response: {:?}", response);

            if let Some(msg_type) = &response.method {
                if msg_type == "window/logMessage" {
                    debug!("Captured log message, continuing to next message");
                    continue;
                } else {
                    debug!("Received non-log message: {}", msg_type);
                    final_response = response;
                    break;
                }
            } else {
                debug!("Received response without method field");
                final_response = response;
                break;
            }
        }

        let result: InitializeResult = match &final_response.result {
            Some(result) => match serde_json::from_value(result.clone()) {
                Ok(r) => r,
                Err(e) => {
                    error!("Failed to parse InitializeResult: {}. Response: {:?}", e, final_response);
                    return Err(format!("Failed to parse InitializeResult: {}. Response: {:?}", e, final_response).into());
                }
            },
            None => {
                error!("No result in initialize response: {:?}", final_response);
                return Err("No result in initialize response".into());
            }
        };

        debug!("Sending initialized notification");
        let notification = self.create_notification("initialized", serde_json::json!({}));
        if let Err(e) = self.send_notification(&notification).await {
            error!("Failed to send initialized notification: {}", e);
            return Err(e);
        }

        debug!("LSP client initialization completed successfully");
        Ok(result)
    }

    fn create_request(&self, method: &str, params: Value) -> String {
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
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

    async fn send_request(&mut self, request: &str) -> Result<(), Box<dyn std::error::Error>> {
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
