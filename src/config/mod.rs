use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;

mod defaults;
pub use defaults::*;

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Server configuration
    #[serde(default)]
    pub server: ServerConfig,

    /// Authentication configuration
    #[serde(default)]
    pub auth: AuthConfig,

    /// UI configuration
    #[serde(default)]
    pub ui: UiConfig,
}

/// Server connection configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Server address
    pub address: String,

    /// Server port
    pub port: u16,

    /// Whether to use TLS
    pub use_tls: bool,

    /// Path to client certificate for mutual TLS
    pub client_cert_path: Option<String>,

    /// Path to client key for mutual TLS
    pub client_key_path: Option<String>,

    /// Whether to verify server certificate
    pub verify_server: bool,
}

/// Authentication configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication method (password, psk, key)
    pub method: String,

    /// Username for authentication
    pub username: Option<String>,

    /// Pre-shared key for authentication
    pub psk: Option<String>,

    /// Whether to save credentials
    pub save_credentials: bool,

    /// Whether to use native OS authentication
    pub use_native_auth: bool,
}

/// UI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiConfig {
    /// Whether to use dark mode
    pub dark_mode: bool,

    /// Whether to start minimized
    pub start_minimized: bool,

    /// Scale factor for UI (1.0 = 100%)
    pub scale_factor: f32,

    /// Custom theme name
    pub theme: Option<String>,
}

/// Load configuration from a file
pub async fn load_config<P: AsRef<Path>>(path: P) -> Result<ClientConfig> {
    // If the file doesn't exist, create it with default values
    if !path.as_ref().exists() {
        let default_config = ClientConfig::default();
        save_config(&path, &default_config).await?;
        return Ok(default_config);
    }

    // Read and parse the config file
    let content = fs::read_to_string(&path)
        .await
        .with_context(|| format!("Failed to read config file: {:?}", path.as_ref()))?;

    toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {:?}", path.as_ref()))
}

/// Save configuration to a file
pub async fn save_config<P: AsRef<Path>>(path: P, config: &ClientConfig) -> Result<()> {
    // Create parent directories if they don't exist
    if let Some(parent) = path.as_ref().parent() {
        fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create config directory: {:?}", parent))?;
    }

    let content = toml::to_string_pretty(config).with_context(|| "Failed to serialize config")?;

    fs::write(&path, content)
        .await
        .with_context(|| format!("Failed to write config file: {:?}", path.as_ref()))
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            auth: AuthConfig::default(),
            ui: UiConfig::default(),
        }
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            address: "127.0.0.1".to_string(),
            port: 8717,
            use_tls: false,
            client_cert_path: None,
            client_key_path: None,
            verify_server: true,
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            method: "password".to_string(),
            username: None,
            psk: None,
            save_credentials: false,
            use_native_auth: false,
        }
    }
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            dark_mode: true,
            start_minimized: false,
            scale_factor: 1.0,
            theme: None,
        }
    }
}
