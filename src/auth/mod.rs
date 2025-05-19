use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::fmt;

mod error;
mod native;
mod password;
mod psk;

pub use error::AuthError;
pub use native::NativeAuthProvider;
pub use password::PasswordAuthProvider;
pub use psk::PskAuthProvider;

/// Authentication method
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMethod {
    /// Username and password authentication
    Password,

    /// Pre-shared key authentication
    Psk,

    /// Native OS authentication
    Native,

    /// Public key authentication
    PublicKey,
}

impl fmt::Display for AuthMethod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuthMethod::Password => write!(f, "password"),
            AuthMethod::Psk => write!(f, "psk"),
            AuthMethod::Native => write!(f, "native"),
            AuthMethod::PublicKey => write!(f, "publickey"),
        }
    }
}

impl AuthMethod {
    /// Parse an authentication method from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "password" => Some(Self::Password),
            "psk" => Some(Self::Psk),
            "native" => Some(Self::Native),
            "publickey" => Some(Self::PublicKey),
            _ => None,
        }
    }
}

/// Authentication credentials
#[derive(Debug, Clone)]
pub enum Credentials {
    /// Username and password
    Password { username: String, password: String },

    /// Pre-shared key
    Psk { key: String },

    /// Native OS credentials
    Native { username: String, token: Vec<u8> },

    /// Public key credentials
    PublicKey {
        username: String,
        signature: Vec<u8>,
    },
}

/// Authentication provider trait
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Get the authentication method for this provider
    fn method(&self) -> AuthMethod;

    /// Authenticate with the server
    async fn authenticate(&self, client: &crate::protocol::Client) -> Result<bool>;

    /// Get authentication credentials
    async fn get_credentials(&self) -> Result<Credentials>;
}

/// Create an authentication provider based on the method
pub fn create_provider(method: AuthMethod, username: &str) -> Box<dyn AuthProvider> {
    match method {
        AuthMethod::Password => Box::new(PasswordAuthProvider::new(username)),
        AuthMethod::Psk => Box::new(PskAuthProvider::new()),
        AuthMethod::Native => Box::new(NativeAuthProvider::new(username)),
        AuthMethod::PublicKey => {
            // Not implemented yet, fall back to password auth
            log::warn!("Public key authentication not implemented yet, falling back to password");
            Box::new(PasswordAuthProvider::new(username))
        }
    }
}
