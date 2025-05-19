//! RCP Client Rust library
//!
//! This is the main library for the RCP client Rust implementation.
//! It provides functionality for authenticating with an RCP server,
//! communicating using the RCP protocol, and displaying a UI.

pub mod auth;
pub mod config;
pub mod protocol;
pub mod resources;
pub mod ui;

use anyhow::Result;
use std::path::Path;

/// Initialize the RCP client with the given configuration file path
/// If the path doesn't exist, a default configuration will be created
pub async fn init_with_config<P: AsRef<Path>>(config_path: P) -> Result<config::ClientConfig> {
    config::load_config(config_path).await
}

/// Connect to an RCP server with the given configuration
pub async fn connect(config: &config::ClientConfig) -> Result<protocol::Client> {
    protocol::Client::connect(&config.server.address, config.server.port).await
}

/// Authenticate with the RCP server
pub async fn authenticate(
    client: &protocol::Client,
    config: &config::ClientConfig,
) -> Result<bool> {
    // Get username for authentication
    let username = config.auth.username.clone().unwrap_or_else(|| {
        // Try to get the current OS username
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string())
    });

    // Determine authentication method
    let auth_method = match auth::AuthMethod::from_str(&config.auth.method) {
        Some(method) => method,
        None => {
            log::warn!(
                "Unknown authentication method: {}, falling back to password",
                config.auth.method
            );
            auth::AuthMethod::Password
        }
    };

    // Authenticate
    log::info!("Authenticating with method: {}", auth_method);
    let auth_provider = auth::create_provider(auth_method, &username);

    client.authenticate_with_provider(&*auth_provider).await
}

/// Start the RCP client UI
pub async fn start_ui(config: config::ClientConfig) -> Result<()> {
    let app = ui::App::new(config)?;
    app.run().await
}
