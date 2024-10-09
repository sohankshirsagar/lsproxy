use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{debug, error, info};
use tokio::time::timeout;
use std::time::Duration;

pub struct LspClient {
    stream: Arc<Mutex<TcpStream>>,
}

impl LspClient {
    pub async fn new(port: u16) -> Result<Self, std::io::Error> {
        let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).await?;
        Ok(LspClient {
            stream: Arc::new(Mutex::new(stream)),
        })
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

        let mut stream = self.stream.lock().await;
        
        stream.write_all(request_str.as_bytes()).await?;
        stream.write_all(b"\r\n").await?;
        stream.flush().await?;

        let mut response = String::new();
        match timeout(Duration::from_secs(30), stream.read_to_string(&mut response)).await {
            Ok(result) => {
                match result {
                    Ok(bytes_read) => {
                        info!("Received LSP response for {}: {} bytes", method, bytes_read);
                        debug!("Response: {}", response);
                    },
                    Err(e) => {
                        error!("Error reading from stream: {}", e);
                        return Err(e.into());
                    }
                }
            },
            Err(_) => {
                error!("Timeout while waiting for LSP server response");
                return Err("LSP server response timeout".into());
            }
        }

        if response.is_empty() {
            return Err("Empty response from LSP server".into());
        }

        let response_json: Value = serde_json::from_str(&response)?;
        Ok(response_json)
    }

    pub async fn send_notification(&self, method: &str, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params
        });

        let notification_str = serde_json::to_string(&notification)?;
        let mut stream = self.stream.lock().await;
        stream.write_all(notification_str.as_bytes()).await?;
        stream.write_all(b"\r\n").await?;
        stream.flush().await?;

        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), std::io::Error> {
        let mut stream = self.stream.lock().await;
        stream.shutdown().await
    }
}
