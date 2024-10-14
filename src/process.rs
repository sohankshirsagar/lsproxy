use log::{debug, error};
use std::error::Error;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

#[async_trait::async_trait]
pub trait Process: Send + Sync {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn receive(&mut self) -> Result<String, Box<dyn Error + Send + Sync>>;
}

pub struct ProcessHandler {
    pub child: Child,
    pub stdin: ChildStdin,
    pub stdout: BufReader<ChildStdout>,
}

impl ProcessHandler {
    pub async fn new(mut child: Child) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        Ok(Self {
            child,
            stdin: stdin,
            stdout: BufReader::new(stdout),
        })
    }
}

#[async_trait::async_trait]
impl Process for ProcessHandler {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending data to process: {}", data);
        self.stdin.write_all(data.as_bytes()).await.unwrap();
        self.stdin.flush().await.unwrap();
        Ok(())
    }

    async fn receive(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut buffer = String::new();
        let bytes_read = self.stdout.read_line(&mut buffer).await?;
        if bytes_read == 0 {
            error!("Process stdout closed unexpectedly");
            return Err("Process stdout closed".into());
        }
        debug!("Received data from process: {}", buffer.trim());
        Ok(buffer)
    }
}
