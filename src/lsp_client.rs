use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use serde_json::Value;
use lsp_types::{InitializeParams, InitializeResult, ClientCapabilities, TextDocumentClientCapabilities, WorkspaceClientCapabilities, WorkspaceFolder};
use log::{error, debug};

pub struct LspClient {
    child: tokio::process::Child,
    stdin: ChildStdin,
    stdout: tokio::io::BufReader<ChildStdout>,
}

impl LspClient {
    pub async fn new(mut child: tokio::process::Child) -> Result<Self, Box<dyn std::error::Error>> {
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stdout = tokio::io::BufReader::new(stdout);

        Ok(LspClient {
            child,
            stdin,
            stdout,
        })
    }

    pub async fn initialize(&mut self, repo_path: Option<String>) -> Result<InitializeResult, Box<dyn std::error::Error>> {
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
        self.send_request(&request).await?;

        let response = match self.read_response().await {
            Ok(resp) => resp,
            Err(e) => {
                error!("Failed to read response: {}", e);
                return Err(e.into());
            }
        };

        debug!("Received response: {:?}", response);

        let result: InitializeResult = match serde_json::from_value(response["result"].clone()) {
            Ok(r) => r,
            Err(e) => {
                error!("Failed to parse InitializeResult: {}. Response: {:?}", e, response);
                return Err(format!("Failed to parse InitializeResult: {}. Response: {:?}", e, response).into());
            }
        };

        // Send initialized notification
        let notification = self.create_notification("initialized", serde_json::json!({}));
        if let Err(e) = self.send_notification(&notification).await {
            error!("Failed to send initialized notification: {}", e);
            return Err(e);
        }

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
        let mut header = String::new();
        let mut content_length = 0;
        let mut timeout = tokio::time::interval(tokio::time::Duration::from_secs(1));

        // Read headers asynchronously with a timeout
        for _ in 0..5 {  // Try for 5 seconds
            tokio::select! {
                result = self.stdout.read_line(&mut header) => {
                    match result {
                        Ok(0) => {
                            debug!("Reached EOF while reading headers");
                            break;
                        }
                        Ok(_) => {
                            debug!("Read header: {}", header.trim());
                            if header.trim().is_empty() {
                                break;
                            }
                            if header.starts_with("Content-Length: ") {
                                content_length = header.trim_start_matches("Content-Length: ").trim().parse()?;
                            }
                            header.clear();
                        }
                        Err(e) => return Err(format!("Error reading header: {}", e).into()),
                    }
                }
                _ = timeout.tick() => {
                    debug!("Timeout while reading headers");
                    break;
                }
            }
        }

        if content_length == 0 {
            debug!("No Content-Length header found. Attempting to read available data.");
            let mut buffer = Vec::new();
            let bytes_read = self.stdout.read_to_end(&mut buffer).await?;
            debug!("Read {} bytes of data without Content-Length", bytes_read);
            if bytes_read == 0 {
                return Err("No data available from LSP server".into());
            }
            let response: Value = serde_json::from_slice(&buffer)
                .map_err(|e| format!("Failed to parse JSON: {}. Content: {}", e, String::from_utf8_lossy(&buffer)))?;
            return Ok(response);
        }

        // Read content asynchronously
        let mut content = vec![0; content_length];
        self.stdout.read_exact(&mut content).await?;

        debug!("Read content: {}", String::from_utf8_lossy(&content));

        let response: Value = serde_json::from_slice(&content)
            .map_err(|e| format!("Failed to parse JSON: {}. Content: {}", e, String::from_utf8_lossy(&content)))?;
        Ok(response)
    }
}
