use anyhow::Result;
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use tokio::time::{timeout, Duration};

mod error;
mod message;
mod response_handler;
mod transport;

pub use error::ProtocolError;
pub use message::{Message, MessageType};
pub use response_handler::handle_response;
pub use transport::Transport;

/// Client connection to the RCP server
pub struct Client {
    /// The underlying transport
    transport: Arc<Mutex<Transport>>,

    /// Channel for receiving messages from the server
    receiver: mpsc::Receiver<Message>,

    /// Channel for sending messages to the server
    sender: mpsc::Sender<Message>,
}

impl Client {
    /// Connect to an RCP server at the given address
    pub async fn connect(address: &str, port: u16) -> Result<Self> {
        // Connect to the server
        let stream = TcpStream::connect(format!("{}:{}", address, port)).await?;

        // Create the transport
        let (transport, receiver, sender) = Transport::new(stream).await?;

        Ok(Self {
            transport,
            receiver,
            sender,
        })
    }

    /// Connect with TLS
    pub async fn connect_tls(
        address: &str,
        port: u16,
        client_cert: Option<&str>,
        client_key: Option<&str>,
        verify_server: bool,
    ) -> Result<Self> {
        // This would use rustls or native-tls to establish a secure connection
        // For now, just use the regular connect and note that TLS would be implemented here
        log::warn!("TLS support not yet implemented, using insecure connection");
        Self::connect(address, port).await
    }

    /// Send a message to the server
    pub async fn send(&self, message: Message) -> Result<()> {
        self.sender
            .send(message)
            .await
            .map_err(|_| ProtocolError::ChannelClosed.into())
    }

    /// Receive a message from the server
    pub async fn receive(&mut self) -> Option<Message> {
        self.receiver.recv().await
    }

    /// Receive a message from the server with timeout
    pub async fn receive_with_timeout(&mut self, timeout_secs: u64) -> Result<Option<Message>> {
        match timeout(Duration::from_secs(timeout_secs), self.receiver.recv()).await {
            Ok(message) => Ok(message),
            Err(_) => Err(ProtocolError::Timeout.into()),
        }
    }

    /// Authenticate with the server
    pub async fn authenticate(
        &self,
        username: &str,
        credentials: &[u8],
        method: &str,
    ) -> Result<bool> {
        let auth_message = Message::new(
            MessageType::Auth,
            serde_json::json!({
                "username": username,
                "credentials": credentials,
                "method": method,
            }),
        );

        // Send the authentication message
        self.send(auth_message).await?;

        // In a real implementation, we would:
        // 1. Wait for a response from the server
        // 2. Check if the response indicates successful authentication
        // 3. Return the result

        // For now, we'll just simulate a successful authentication
        log::info!(
            "Sent authentication request for user: {}, method: {}",
            username,
            method
        );

        // Simulate a delay for authentication processing
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Always return success for now
        Ok(true)
    }

    /// Authenticate with the server using an authentication provider
    pub async fn authenticate_with_provider(
        &self,
        provider: &dyn crate::auth::AuthProvider,
    ) -> Result<bool> {
        provider.authenticate(self).await
    }

    /// Close the connection
    pub async fn close(self) -> Result<()> {
        // This will drop the sender and receiver channels
        // The transport will be dropped when the last reference is gone
        Ok(())
    }
}
