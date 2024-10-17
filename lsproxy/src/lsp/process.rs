use log::{debug, error, warn};
use std::error::Error;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout};

#[async_trait::async_trait]
pub trait Process: Send + Sync {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>>;
    async fn receive(&mut self) -> Result<String, Box<dyn Error + Send + Sync>>;
}

pub struct ProcessHandler {
    pub stdin: ChildStdin,
    pub stdout: BufReader<ChildStdout>,
}

impl ProcessHandler {
    pub async fn new(mut child: Child) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;
        Ok(Self {
            stdin: stdin,
            stdout: BufReader::new(stdout),
        })
    }
}

#[async_trait::async_trait]
impl Process for ProcessHandler {
    async fn send(&mut self, data: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        debug!("Sending data to process:\n{}", data);
        println!("Trying to figure out why this has weird spacing in it:\n{}",data);
        self.stdin.write_all(data.as_bytes()).await?;
        self.stdin.flush().await?;
        Ok(())
    }

    async fn receive(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut content_length: Option<usize> = None;
        let mut buffer = Vec::new();

        loop {
            let timeout = tokio::time::sleep(Duration::from_secs(5));
            tokio::select! {
                result = self.stdout.read_until(b'\n', &mut buffer) => {
                    match result {
                        Ok(0) => {
                            debug!("Reached EOF");
                            return Err("Unexpected EOF".into());
                        }
                        Ok(n) => {
                            let line = String::from_utf8_lossy(&buffer[buffer.len() - n..]);
                            if line.trim().is_empty() && content_length.is_some() {
                                let length = content_length.unwrap();
                                let mut content = vec![0; length];
                                self.stdout.read_exact(&mut content).await?;
                                return Ok(String::from_utf8(content)?);
                            } else if line.starts_with("Content-Length: ") {
                                content_length = Some(line.trim_start_matches("Content-Length: ").trim().parse()?);
                                debug!("Content-Length found: {:?}", content_length);
                            } else {
                                debug!("Received non-content line: {}", line.trim());
                            }
                        }
                        Err(e) => {
                            error!("Error reading from stdout: {}", e);
                            return Err(e.into());
                        }
                    }
                    buffer.clear();
                }
                _ = timeout => {
                    warn!("Timeout while reading response");
                    return Err("Timeout while reading response".into());
                }
            }
        }
    }
}
