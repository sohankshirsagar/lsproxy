use std::process::{Child, Stdio};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use jsonrpc::Client;
use lsp_types::{
    InitializeParams, InitializeResult, ClientCapabilities,
    TextDocumentClientCapabilities, WorkspaceClientCapabilities,
};
use serde_json::Value;

pub struct LspClient {
    process: Child,
    client: Client,
}

impl LspClient {
    pub async fn new(mut process: Child) -> std::io::Result<Self> {
        let stdin = process.stdin.take().expect("Failed to get stdin");
        let stdout = process.stdout.take().expect("Failed to get stdout");

        let writer = tokio::io::BufWriter::new(stdin);
        let reader = BufReader::new(stdout);

        let (client, _) = Client::new(reader, writer);

        Ok(Self { process, client })
    }

    pub async fn initialize(&self, root_uri: Option<String>) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        let params = InitializeParams {
            process_id: Some(std::process::id()),
            root_uri: root_uri.map(|uri| url::Url::parse(&uri).unwrap()),
            capabilities: ClientCapabilities {
                text_document: Some(TextDocumentClientCapabilities::default()),
                workspace: Some(WorkspaceClientCapabilities::default()),
                ..Default::default()
            },
            ..Default::default()
        };

        let result: Value = self.client.request("initialize", params).await?;
        let initialize_result: InitializeResult = serde_json::from_value(result)?;

        self.client.notify("initialized", serde_json::json!({})).await?;

        Ok(initialize_result)
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error>> {
        self.client.request::<(), ()>("shutdown", ()).await?;
        self.client.notify("exit", ()).await?;
        Ok(())
    }
}
