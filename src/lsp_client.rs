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

    pub async fn send_request(&mut self, method: &str, params: Value) -> Result<Value, std::io::Error> {
        let request = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": method,
            "params": params
        });

        let request_str = serde_json::to_string(&request)?;
        self.process.stdin.as_mut().unwrap().write_all(request_str.as_bytes()).await?;

        let mut response = String::new();
        self.process.stdout.as_mut().unwrap().read_to_string(&mut response).await?;

        let response_json: Value = serde_json::from_str(&response)?;
        Ok(response_json)
    }
}
