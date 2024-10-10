use std::process::{Child};
use std::io::{BufReader, BufWriter, Write};
use serde_json::{json, Value};
use lsp_types::{
    InitializeParams, InitializedParams, InitializeResult, 
    ClientCapabilities, TextDocumentClientCapabilities,
    WorkspaceClientCapabilities, request::Request,
};
use futures::channel::mpsc::{channel, Sender, Receiver};
use futures::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

pub struct LspClient {
    process: Child,
    request_id: u64,
    writer: Sender<String>,
    reader: Receiver<String>,
}

impl LspClient {
    pub fn new(mut process: Child) -> std::io::Result<Self> {
        let stdin = process.stdin.take().expect("Failed to get stdin");
        let stdout = process.stdout.take().expect("Failed to get stdout");
        
        let (tx_writer, mut rx_writer) = channel::<String>(32);
        let (mut tx_reader, rx_reader) = channel::<String>(32);

        tokio::spawn(async move {
            let mut writer = BufWriter::new(stdin);
            while let Some(message) = rx_writer.next().await {
                writer.write_all(message.as_bytes()).await.expect("Failed to write to stdin");
                writer.flush().await.expect("Failed to flush stdin");
            }
        });

        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            let mut buffer = String::new();
            loop {
                buffer.clear();
                if let Ok(size) = reader.read_line(&mut buffer).await {
                    if size == 0 {
                        break;
                    }
                    tx_reader.send(buffer.clone()).await.expect("Failed to send message");
                }
            }
        });

        Ok(Self { 
            process,
            request_id: 0,
            writer: tx_writer,
            reader: rx_reader,
        })
    }

    async fn send_request<R: Request>(&mut self, params: R::Params) -> Result<R::Result, Box<dyn std::error::Error>> 
    where
        R::Params: serde::Serialize,
        R::Result: serde::de::DeserializeOwned,
    {
        self.request_id += 1;
        let request = json!({
            "jsonrpc": "2.0",
            "id": self.request_id,
            "method": R::METHOD,
            "params": params,
        });
        
        let message = format!("Content-Length: {}\r\n\r\n{}", 
            request.to_string().len(),
            request.to_string()
        );

        self.writer.send(message).await?;

        while let Some(response) = self.reader.next().await {
            if response.contains("Content-Length") {
                continue;
            }
            let parsed: Value = serde_json::from_str(&response)?;
            if let Some(id) = parsed.get("id") {
                if id.as_u64() == Some(self.request_id) {
                    return Ok(serde_json::from_value(parsed["result"].clone())?);
                }
            }
        }

        Err("No response received".into())
    }

    async fn send_notification<N: lsp_types::notification::Notification>(&mut self, params: N::Params) -> Result<(), Box<dyn std::error::Error>> 
    where
        N::Params: serde::Serialize,
    {
        let notification = json!({
            "jsonrpc": "2.0",
            "method": N::METHOD,
            "params": params,
        });
        
        let message = format!("Content-Length: {}\r\n\r\n{}", 
            notification.to_string().len(),
            notification.to_string()
        );

        self.writer.send(message).await?;
        Ok(())
    }

    pub async fn initialize(&mut self, root_uri: Option<String>) -> Result<InitializeResult, Box<dyn std::error::Error>> {
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

        let result: InitializeResult = self.send_request::<lsp_types::request::Initialize>(params).await?;

        self.send_notification::<lsp_types::notification::Initialized>(InitializedParams {}).await?;

        Ok(result)
    }

    pub async fn shutdown(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        self.send_request::<lsp_types::request::Shutdown>(()).await?;
        self.send_notification::<lsp_types::notification::Exit>(()).await?;
        Ok(())
    }
}
