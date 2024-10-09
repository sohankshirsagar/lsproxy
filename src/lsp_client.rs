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
        debug!("Sending LSP request: {}", request_str);

        let mut stream = self.stream.lock().await;
        
        stream.write_all(request_str.as_bytes()).await?;
        debug!("Request sent, waiting for response");

        let mut response = String::new();
        stream.read_to_string(&mut response).await?;
        debug!("Received response: {}", response);

        let response_json: Value = serde_json::from_str(&response)?;
        Ok(response_json)
    }

    pub async fn shutdown(&self) -> Result<(), std::io::Error> {
        let mut stream = self.stream.lock().await;
        stream.shutdown().await
    }
}
