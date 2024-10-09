use tokio::process::ChildStdin;
use tokio::process::ChildStdout;
use tokio::io::{AsyncReadExt, AsyncWriteExt, BufReader, AsyncBufReadExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{debug, error, info};
use tokio::time::timeout;
use std::time::Duration;

pub struct LspClient {
    stdin: Arc<Mutex<ChildStdin>>,
    stdout: Arc<Mutex<BufReader<ChildStdout>>>,
}

impl LspClient {
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        LspClient {
            stdin: Arc::new(Mutex::new(stdin)),
            stdout: Arc::new(Mutex::new(BufReader::new(stdout))),
        }
    }

    pub async fn initialize(&self, root_uri: &str) -> Result<Value, Box<dyn std::error::Error>> {
        info!("Initializing LSP for root_uri: {}", root_uri);
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                // Specify client capabilities here
            }
        });

        // Increase timeout to 60 seconds for initialization
        match timeout(Duration::from_secs(60), self.send_request("initialize", params)).await {
            Ok(result) => {
                match result {
                    Ok(response) => {
                        info!("LSP initialization successful");
                        // Handle potential log messages during initialization
                        if response.get("method") == Some(&json!("window/logMessage")) {
                            warn!("Received log message during initialization: {:?}", response);
                            // You might want to add additional logic here to handle log messages
                        }
                        // After successful initialize, send the initialized notification
                        self.send_notification("initialized", json!({})).await?;
                        Ok(response)
                    },
                    Err(e) => {
                        error!("LSP initialization failed: {}", e);
                        Err(e)
                    }
                }
            },
            Err(_) => {
                error!("LSP initialization timed out after 60 seconds");
                Err("LSP initialization timeout".into())
            }
        }
    }

    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let request_str = serde_json::to_string(&request)?;
        info!("Sending LSP request: {}", method);
        debug!("Request payload: {}", request_str);

        let mut stdin = self.stdin.lock().await;
        stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", request_str.len(), request_str).as_bytes()).await?;
        stdin.flush().await?;

        let mut stdout = self.stdout.lock().await;
        let mut headers = String::new();
        let mut content_length = None;

        // Read headers
        loop {
            let mut line = String::new();
            stdout.read_line(&mut line).await?;
            if line == "\r\n" {
                break;
            }
            headers.push_str(&line);
            if line.starts_with("Content-Length: ") {
                content_length = Some(line.trim_start_matches("Content-Length: ").trim().parse::<usize>()?);
            }
        }

        // Read content
        let content_length = content_length.ok_or("No Content-Length header found")?;
        let mut content = vec![0; content_length];
        stdout.read_exact(&mut content).await?;

        let response_str = String::from_utf8(content)?;
        debug!("Response: {}", response_str);

        let response_json: Value = serde_json::from_str(&response_str)?;
        Ok(response_json)
    }

    pub async fn send_notification(&self, method: &str, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let notification_str = serde_json::to_string(&notification)?;
        let mut stdin = self.stdin.lock().await;
        stdin.write_all(format!("Content-Length: {}\r\n\r\n{}", notification_str.len(), notification_str).as_bytes()).await?;
        stdin.flush().await?;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Send shutdown request
        self.send_request("shutdown", json!(null)).await?;
        // Send exit notification
        self.send_notification("exit", json!(null)).await?;
        Ok(())
    }
}
