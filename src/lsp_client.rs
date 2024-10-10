use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use serde_json::Value;
use lsp_types::{InitializeParams, InitializeResult, ClientCapabilities, TextDocumentClientCapabilities, WorkspaceClientCapabilities, WorkspaceFolder};
use log::{error, debug, warn};
use tokio::time::Duration;

pub struct LspClient {
    child: tokio::process::Child,
    stdin: ChildStdin,
    stdout: tokio::io::BufReader<ChildStdout>,
}

impl LspClient {
    pub async fn new(mut child: tokio::process::Child) -> Result<Self, Box<dyn std::error::Error>> {
        debug!("Creating new LspClient");
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stdout = tokio::io::BufReader::new(stdout);

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

        debug!("Waiting for initialize response...");
        let response = match self.read_response().await {
            Ok(resp) => resp,
            Err(e) => {
                error!("Failed to read initialize response: {}", e);
                return Err(e.into());
            }
        };

        debug!("Received initialize response: {:?}", response);

        let result: InitializeResult = match serde_json::from_value(response["result"].clone()) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse InitializeResult: {}. Response: {:?}", e, response);
                return Err(format!("Failed to parse InitializeResult: {}. Response: {:?}", e, response).into());
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

    async fn read_response(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        debug!("Starting to read response from LSP server");
        let mut header = String::new();
        let mut content_length = 0;
        let mut timeout = tokio::time::interval(Duration::from_secs(5));

        // Read headers asynchronously with a timeout
        for i in 0..12 {  // Try for 60 seconds (5 seconds * 12 attempts)
            debug!("Attempt {} to read headers", i + 1);
            tokio::select! {
                result = self.stdout.read_line(&mut header) => {
                    match result {
                        Ok(0) => {
                            warn!("Reached EOF while reading headers");
                            break;
                        }
                        Ok(n) => {
                            debug!("Read header ({} bytes): {}", n, header.trim());
                            if header.trim().is_empty() {
                                debug!("Empty line found, headers complete");
                                break;
                            }
                            if header.starts_with("Content-Length: ") {
                                content_length = header.trim_start_matches("Content-Length: ").trim().parse()?;
                                debug!("Content-Length found: {}", content_length);
                            }
                            header.clear();
                        }
                        Err(e) => {
                            error!("Error reading header: {}", e);
                            return Err(format!("Error reading header: {}", e).into());
                        }
                    }
                }
                _ = timeout.tick() => {
                    warn!("Timeout while reading headers");
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

        if content_length == 0 {
            warn!("No Content-Length header found. Attempting to read available data.");
            let mut buffer = Vec::new();
            let bytes_read = self.stdout.read_to_end(&mut buffer).await?;
            debug!("Read {} bytes of data without Content-Length", bytes_read);
            if bytes_read == 0 {
                error!("No data available from LSP server");
                return Err("No data available from LSP server".into());
            }
            debug!("Attempting to parse JSON from buffer");
            let response: Value = serde_json::from_slice(&buffer)
                .map_err(|e| format!("Failed to parse JSON: {}. Content: {}", e, String::from_utf8_lossy(&buffer)))?;
            return Ok(response);
        }

        debug!("Reading content with length: {}", content_length);
        // Read content asynchronously with a timeout
        let mut content = vec![0; content_length];
        let read_result = tokio::time::timeout(Duration::from_secs(30), self.stdout.read_exact(&mut content)).await;
        match read_result {
            Ok(Ok(_)) => {
                debug!("Read content: {}", String::from_utf8_lossy(&content));
                debug!("Parsing JSON content");
                let response: Value = serde_json::from_slice(&content)
                    .map_err(|e| format!("Failed to parse JSON: {}. Content: {}", e, String::from_utf8_lossy(&content)))?;
                debug!("Successfully parsed JSON response");
                Ok(response)
            }
            Ok(Err(e)) => {
                error!("Error reading content: {}", e);
                Err(format!("Error reading content: {}", e).into())
            }
            Err(_) => {
                error!("Timeout while reading content");
                Err("Timeout while reading content".into())
            }
        }
    }
}
