use jsonrpc::{Client, Request};
use tokio::process::{ChildStdin, ChildStdout};
use tokio::io::BufReader;
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::Mutex;
use lsp_types::{
    InitializeParams, InitializeResult, ServerCapabilities,
    notification::{Notification, Exit},
    request::{Request as LspRequest, Initialize, Shutdown},
};

pub struct LspClient {
    client: Arc<Mutex<Client>>,
    capabilities: Arc<Mutex<Option<ServerCapabilities>>>,
}

impl LspClient {
    // Create a new LspClient instance
    pub fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        let client = Client::new();
        client.start(stdin, BufReader::new(stdout));
        LspClient {
            client: Arc::new(Mutex::new(client)),
            capabilities: Arc::new(Mutex::new(None)),
        }
    }

    // Initialize the LSP connection
    pub async fn initialize(&self, root_uri: &str) -> Result<(), Box<dyn std::error::Error>> {
        let params = InitializeParams {
            root_uri: Some(root_uri.parse()?),
            ..InitializeParams::default()
        };

        let request = Request::new(Initialize::METHOD, serde_json::to_value(params)?);
        let response: InitializeResult = self.client.lock().await.send_request(request).await?;

        *self.capabilities.lock().await = Some(response.capabilities);
        Ok(())
    }

    // Get the server capabilities
    pub async fn get_capabilities(&self) -> Option<ServerCapabilities> {
        self.capabilities.lock().await.clone()
    }

    // Send a request to the LSP server
    pub async fn send_request(&self, method: &str, params: Value) -> Result<Value, Box<dyn std::error::Error>> {
        let request = Request::new(method, params);
        Ok(self.client.lock().await.send_request(request).await?)
    }


    // Send a notification to the LSP server
    pub async fn send_notification(&self, method: &str, params: Value) -> Result<(), Box<dyn std::error::Error>> {
        let notification = Request::notification(method, params);
        Ok(self.client.lock().await.send_notification(notification).await?)
    }

    // Shutdown the LSP connection
    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        // Send shutdown request
        let shutdown_request = Request::new(Shutdown::METHOD, serde_json::Value::Null);
        self.client.lock().await.send_request(shutdown_request).await?;

        // Send exit notification
        let exit_notification = Request::notification(Exit::METHOD, serde_json::Value::Null);
        self.client.lock().await.send_notification(exit_notification).await?;

        Ok(())
    }
}
