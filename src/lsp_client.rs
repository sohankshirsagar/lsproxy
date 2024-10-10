use std::process::Child;
use std::io::{BufReader, BufWriter};
use lsp_types::{InitializeParams, InitializeResult};
use jsonrpc::{Client, Transport};
use serde_json::Value;

pub struct LspClient {
    process: Child,
    rpc_client: Client,
}

impl LspClient {
    pub fn new(mut process: Child) -> std::io::Result<Self> {
        let stdin = process.stdin.take().expect("Failed to get stdin");
        let stdout = process.stdout.take().expect("Failed to get stdout");
        
        let transport = Transport::new(
            BufReader::new(stdout),
            BufWriter::new(stdin),
        );
        
        let rpc_client = Client::with_transport(transport);
        
        Ok(Self { process, rpc_client })
    }

    pub async fn initialize(&mut self, root_uri: Option<String>) -> Result<InitializeResult, Box<dyn std::error::Error>> {
        let params = InitializeParams {
            root_uri: root_uri.map(|uri| url::Url::parse(&uri).map_err(|e| format!("Invalid root URI: {}", e))?),
            capabilities: ClientCapabilities {
                // Fill in the capabilities your client supports
                ..Default::default()
            },
            // Add other necessary fields
            ..Default::default()
        };

        let initialize_result: InitializeResult = self.rpc_client
            .request("initialize", &params)
            .await
            .map_err(|e| format!("Failed to initialize language server: {}", e))?;

        self.rpc_client.notify("initialized", InitializedParams {})
            .await
            .map_err(|e| format!("Failed to send initialized notification: {}", e))?;

        Ok(initialize_result)
    }
}
