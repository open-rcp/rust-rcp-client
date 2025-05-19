use crate::auth::{AuthError, AuthMethod, AuthProvider, Credentials};
use crate::protocol::Client;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// Pre-shared key authentication provider
pub struct PskAuthProvider {
    key: Option<String>,
}

impl PskAuthProvider {
    /// Create a new PSK authentication provider
    pub fn new() -> Self {
        Self { key: None }
    }

    /// Set the pre-shared key
    pub fn with_key(mut self, key: &str) -> Self {
        self.key = Some(key.to_string());
        self
    }

    /// Try to load the PSK from the keyring
    async fn load_key_from_keyring(&self) -> Result<Option<String>, keyring::Error> {
        let service = "rcp-client";
        let username = "psk"; // Using "psk" as the username for the keyring

        let entry = keyring::Entry::new(service, username)?;
        match entry.get_password() {
            Ok(key) => Ok(Some(key)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Save the PSK to the keyring
    async fn save_key_to_keyring(&self, key: &str) -> Result<(), keyring::Error> {
        let service = "rcp-client";
        let username = "psk"; // Using "psk" as the username for the keyring

        let entry = keyring::Entry::new(service, username)?;
        entry.set_password(key)
    }

    /// Prompt the user for a PSK
    async fn prompt_for_key(&self) -> Result<String, AuthError> {
        // In a real implementation, this would show a GUI dialog
        // For now, just return an error
        Err(AuthError::Other("PSK dialog not implemented".to_string()))
    }
}

#[async_trait]
impl AuthProvider for PskAuthProvider {
    fn method(&self) -> AuthMethod {
        AuthMethod::Psk
    }

    async fn authenticate(&self, client: &Client) -> Result<bool> {
        let credentials = self.get_credentials().await?;

        // Extract PSK
        let key = match credentials {
            Credentials::Psk { key } => key,
            _ => return Err(AuthError::InvalidCredentials.into()),
        };

        // Send authentication message
        let auth_message = crate::protocol::Message::new(
            crate::protocol::MessageType::Auth,
            json!({
                "credentials": key,
                "method": "psk",
            }),
        );

        client.send(auth_message).await?;

        // Wait for response with a timeout
        // TODO: Implement response handling in the Client
        // For now, assume authentication was successful

        Ok(true)
    }

    async fn get_credentials(&self) -> Result<Credentials> {
        // If we already have a key, use it
        if let Some(key) = &self.key {
            return Ok(Credentials::Psk { key: key.clone() });
        }

        // Try to get the key from the keyring
        match self.load_key_from_keyring().await {
            Ok(Some(key)) => {
                return Ok(Credentials::Psk { key });
            }
            Ok(None) => {
                // Prompt the user for a key
                let key = self.prompt_for_key().await?;
                Ok(Credentials::Psk { key })
            }
            Err(e) => Err(AuthError::KeyringError(e).into()),
        }
    }
}
