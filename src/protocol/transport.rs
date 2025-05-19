use crate::protocol::{Message, ProtocolError};
use anyhow::Result;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, Mutex};

/// Transport layer for RCP protocol
pub struct Transport {
    /// The underlying TCP stream
    stream: TcpStream,

    /// Buffer for reading
    read_buffer: Vec<u8>,
}

impl Transport {
    /// Create a new transport using the given stream
    pub async fn new(
        stream: TcpStream,
    ) -> Result<(
        Arc<Mutex<Self>>,
        mpsc::Receiver<Message>,
        mpsc::Sender<Message>,
    )> {
        // Create channels for sending and receiving messages
        let (incoming_tx, incoming_rx) = mpsc::channel(100);
        let (outgoing_tx, mut outgoing_rx) = mpsc::channel(100);

        // Create the transport
        let transport = Arc::new(Mutex::new(Self {
            stream,
            read_buffer: Vec::with_capacity(4096),
        }));

        // Spawn a task to receive messages from the stream
        let transport_clone = transport.clone();
        tokio::spawn(async move {
            let mut transport = transport_clone.lock().await;

            loop {
                // Read a message from the stream
                match transport.read_message().await {
                    Ok(message) => {
                        // Send the message to the incoming channel
                        if incoming_tx.send(message).await.is_err() {
                            // The receiver was dropped, so we exit
                            break;
                        }
                    }
                    Err(e) => {
                        log::error!("Error reading message: {}", e);
                        break;
                    }
                }
            }
        });

        // Spawn a task to send messages to the stream
        let transport_clone = transport.clone();
        tokio::spawn(async move {
            while let Some(message) = outgoing_rx.recv().await {
                let mut transport = transport_clone.lock().await;

                if let Err(e) = transport.write_message(&message).await {
                    log::error!("Error writing message: {}", e);
                    break;
                }
            }
        });

        Ok((transport, incoming_rx, outgoing_tx))
    }

    /// Read a message from the stream
    async fn read_message(&mut self) -> Result<Message> {
        // Read message size (4 bytes)
        let mut size_buf = [0u8; 4];
        self.stream.read_exact(&mut size_buf).await?;
        let size = u32::from_be_bytes(size_buf) as usize;

        // Ensure the buffer is large enough
        if self.read_buffer.len() < size {
            self.read_buffer.resize(size, 0);
        }

        // Read the message data
        self.stream
            .read_exact(&mut self.read_buffer[..size])
            .await?;

        // Parse the message
        let message = serde_json::from_slice(&self.read_buffer[..size])
            .map_err(|e| ProtocolError::MalformedPayload(e.to_string()))?;

        Ok(message)
    }

    /// Write a message to the stream
    async fn write_message(&mut self, message: &Message) -> Result<()> {
        // Serialize the message
        let data = serde_json::to_vec(message)
            .map_err(|e| ProtocolError::MalformedPayload(e.to_string()))?;

        // Write the message size
        let size = data.len() as u32;
        self.stream.write_all(&size.to_be_bytes()).await?;

        // Write the message data
        self.stream.write_all(&data).await?;

        Ok(())
    }
}
