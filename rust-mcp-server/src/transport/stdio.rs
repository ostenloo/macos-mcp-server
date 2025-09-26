use async_trait::async_trait;
use tokio::io::{self, AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};

use super::Transport;

/// Transport implementation that uses stdin/stdout with Content-Length framing.
pub struct StdioTransport {
    reader: BufReader<Stdin>,
    writer: Stdout,
    buffer: Vec<u8>,
}

impl StdioTransport {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
            writer: io::stdout(),
            buffer: Vec::with_capacity(8 * 1024),
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn read(&mut self) -> anyhow::Result<Option<String>> {
        let mut header_line = String::new();
        let mut content_length: Option<usize> = None;

        loop {
            header_line.clear();
            let bytes = self.reader.read_line(&mut header_line).await?;
            if bytes == 0 {
                // EOF encountered.
                return Ok(None);
            }

            if header_line == "\r\n" {
                break;
            }

            let trimmed = header_line.trim();
            if let Some(rest) = trimmed.strip_prefix("Content-Length:") {
                content_length = Some(rest.trim().parse()?);
            }
        }

        let len = content_length.ok_or_else(|| anyhow::anyhow!("missing Content-Length header"))?;
        self.buffer.resize(len, 0);
        self.reader.read_exact(&mut self.buffer).await?;

        // Consume the trailing CRLF after the JSON payload per header-based framing convention.
        let mut trailing = [0u8; 2];
        self.reader.read_exact(&mut trailing).await?;

        let payload = String::from_utf8(self.buffer.clone())?;
        self.buffer.clear();
        Ok(Some(payload))
    }

    async fn write(&mut self, payload: &str) -> anyhow::Result<()> {
        let bytes = payload.as_bytes();
        let header = format!("Content-Length: {}\r\n\r\n", bytes.len());
        self.writer.write_all(header.as_bytes()).await?;
        self.writer.write_all(bytes).await?;
        self.writer.flush().await?;
        Ok(())
    }
}
