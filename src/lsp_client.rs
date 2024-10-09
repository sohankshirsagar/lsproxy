use tokio::net::TcpStream;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;
use log::debug;

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
        let bytes_read = stream.read_to_string(&mut response).await?;
        debug!("Received response ({} bytes): {}", bytes_read, response);

        if response.is_empty() {
            return Err("Empty response from LSP server".into());
        }

        debug!("Parsing response as JSON");
        let response_json: Value = serde_json::from_str(&response)?;
        debug!("Parsed response: {:?}", response_json);
        Ok(response_json)
    }

    pub async fn shutdown(&self) -> Result<(), std::io::Error> {
        let mut stream = self.stream.lock().await;
        stream.shutdown().await
    }
}
