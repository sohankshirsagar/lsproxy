use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::{debug, error};
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
        let params = json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "capabilities": {
                // Specify client capabilities here
            }
        });

        let response = self.send_request("initialize", params).await?;

        // After successful initialize, send the initialized notification
        self.send_notification("initialized", json!({})).await?;

        Ok(response)
    }

    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let request_str = serde_json::to_string(&request)?;
        debug!("Preparing to send LSP request: {}", request_str);

        let mut stream = self.stream.lock().await;
        debug!("Acquired stream lock");
        
        debug!("Writing request to stream");
        stream.write_all(request_str.as_bytes()).await?;
        stream.write_all(b"\r\n").await?;  // Add newline to end the request
        debug!("Request sent, flushing stream");
        stream.flush().await?;

        debug!("Reading response");
        let mut response = String::new();
        match timeout(Duration::from_secs(30), stream.read_to_string(&mut response)).await {
            Ok(result) => {
                match result {
                    Ok(bytes_read) => {
                        debug!("Received response ({} bytes): {}", bytes_read, response);
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

        debug!("Parsing response as JSON");
        let response_json: Value = serde_json::from_str(&response)?;
        debug!("Parsed response: {:?}", response_json);
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
