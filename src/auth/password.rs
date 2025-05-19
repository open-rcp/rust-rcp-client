use crate::auth::{AuthError, AuthMethod, AuthProvider, Credentials};
use crate::protocol::Client;
use anyhow::Result;
use async_trait::async_trait;
use serde_json::json;

/// Password authentication provider
pub struct PasswordAuthProvider {
    username: String,
    password: Option<String>,
}

impl PasswordAuthProvider {
    /// Create a new password authentication provider
    pub fn new(username: &str) -> Self {
        Self {
            username: username.to_string(),
            password: None,
        }
    }

    /// Set the password for this provider
    pub fn with_password(mut self, password: &str) -> Self {
        self.password = Some(password.to_string());
        self
    }

    /// Get the password from the keyring if available
    async fn get_password_from_keyring(&self) -> Result<Option<String>, keyring::Error> {
        let service = "rcp-client";
        let entry = keyring::Entry::new(service, &self.username)?;
        match entry.get_password() {
            Ok(password) => Ok(Some(password)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(e),
        }
    }

    /// Save the password to the keyring
    async fn save_password_to_keyring(&self, password: &str) -> Result<(), keyring::Error> {
        let service = "rcp-client";
        let entry = keyring::Entry::new(service, &self.username)?;
        entry.set_password(password)
    }

    /// Prompt the user for a password
    async fn prompt_for_password(&self) -> Result<String, AuthError> {
        // In a real implementation, this would show a GUI dialog
        // For now, just return an error
        Err(AuthError::Other(
            "Password dialog not implemented".to_string(),
        ))
    }
}

#[async_trait]
impl AuthProvider for PasswordAuthProvider {
    fn method(&self) -> AuthMethod {
        AuthMethod::Password
    }

    async fn authenticate(&self, client: &Client) -> Result<bool> {
        let credentials = self.get_credentials().await?;

        // Extract username and password
        let (username, password) = match credentials {
            Credentials::Password { username, password } => (username, password),
            _ => return Err(AuthError::InvalidCredentials.into()),
        };

        // Send authentication message
        let auth_message = crate::protocol::Message::new(
            crate::protocol::MessageType::Auth,
            json!({
                "username": username,
                "credentials": password,
                "method": "password",
            }),
        );

        client.send(auth_message).await?;

        // Wait for response with a timeout
        // TODO: Implement response handling in the Client
        // For now, assume authentication was successful

        Ok(true)
    }

    async fn get_credentials(&self) -> Result<Credentials> {
        // If we already have a password, use it
        if let Some(password) = &self.password {
            return Ok(Credentials::Password {
                username: self.username.clone(),
                password: password.clone(),
            });
        }

        // Try to get the password from the keyring
        match self.get_password_from_keyring().await {
            Ok(Some(password)) => {
                return Ok(Credentials::Password {
                    username: self.username.clone(),
                    password,
                });
            }
            Ok(None) => {
                // Prompt the user for a password
                let password = self.prompt_for_password().await?;
                Ok(Credentials::Password {
                    username: self.username.clone(),
                    password,
                })
            }
            Err(e) => Err(AuthError::KeyringError(e).into()),
        }
    }
}
