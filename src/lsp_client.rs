use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout};
use serde_json::Value;
use lsp_types::{
    InitializeParams, InitializeResult, ClientCapabilities,
    TextDocumentClientCapabilities, WorkspaceClientCapabilities,
};

pub struct LspClient {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl LspClient {
    pub fn new(mut child: Child) -> Result<Self, Box<dyn std::error::Error>> {
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        let stdout = BufReader::new(stdout);

        Ok(LspClient {
            child,
            stdin,
            stdout,
        })
    }

    pub fn init(&mut self) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        let capabilities = ClientCapabilities {
            text_document: Some(TextDocumentClientCapabilities::default()),
            workspace: Some(WorkspaceClientCapabilities::default()),
            ..Default::default()
        };

        let params = InitializeParams {
            capabilities,
            ..Default::default()
        };

        let request = self.create_request("initialize", serde_json::to_value(params)?);
        self.send_request(&request)?;

        let response = self.read_response()?;
        let result: InitializeResult = serde_json::from_value(response["result"].clone())?;

        // Send initialized notification
        let notification = self.create_notification("initialized", serde_json::json!({}));
        self.send_notification(&notification)?;

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

    fn send_request(&mut self, request: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content_length = request.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_length);
        
        self.stdin.write_all(header.as_bytes())?;
        self.stdin.write_all(request.as_bytes())?;
        self.stdin.flush()?;

        Ok(())
    }

    fn send_notification(&mut self, notification: &str) -> Result<(), Box<dyn std::error::Error>> {
        let content_length = notification.len();
        let header = format!("Content-Length: {}\r\n\r\n", content_length);
        
        self.stdin.write_all(header.as_bytes())?;
        self.stdin.write_all(notification.as_bytes())?;
        self.stdin.flush()?;

        Ok(())
    }

    fn read_response(&mut self) -> Result<Value, Box<dyn std::error::Error>> {
        let mut header = String::new();
        let mut content_length = 0;

        // Read headers
        loop {
            self.stdout.read_line(&mut header)?;
            if header.trim().is_empty() {
                break;
            }
            if header.starts_with("Content-Length: ") {
                content_length = header.trim_start_matches("Content-Length: ").trim().parse()?;
            }
            header.clear();
        }

        // Read content
        let mut content = vec![0; content_length];
        self.stdout.read_exact(&mut content)?;

        let response: Value = serde_json::from_slice(&content)?;
        Ok(response)
    }
}
