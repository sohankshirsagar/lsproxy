use tokio::process::Child;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use serde_json::{json, Value};

pub struct LspClient {
    process: Child,
}

impl LspClient {
    pub fn new(process: Child) -> Self {
        LspClient { process }
    }

    pub async fn send_request(&mut self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let request_str = serde_json::to_string(&request)?;
        
        let stdin = self.process.stdin.as_mut().ok_or("Failed to get stdin")?;
        stdin.write_all(request_str.as_bytes()).await?;

        let mut response = String::new();
        let stdout = self.process.stdout.as_mut().ok_or("Failed to get stdout")?;
        stdout.read_to_string(&mut response).await?;

        let response_json: Value = serde_json::from_str(&response)?;
        Ok(response_json)
    }

    pub async fn kill(&mut self) -> Result<(), std::io::Error> {
        self.process.kill().await
    }
}
