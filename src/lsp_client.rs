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
            root_uri: root_uri.map(|uri| url::Url::parse(&uri).unwrap()),
            ..Default::default()
        };

        let result: Value = self.rpc_client.request("initialize", &params).await?;
        let initialize_result: InitializeResult = serde_json::from_value(result)?;

        Ok(initialize_result)
    }
}
