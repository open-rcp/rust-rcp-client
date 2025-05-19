use crate::auth;
use crate::config::ClientConfig;
use crate::protocol;
use anyhow::Result;
use log::{error, info, warn};
use tokio::sync::oneshot;

mod event_app;
pub use event_app::EventBasedApp;

/// Main application UI
pub struct App {
    /// Application configuration
    config: ClientConfig,
}

impl App {
    /// Create a new application with the given configuration
    pub fn new(config: ClientConfig) -> Result<Self> {
        Ok(Self { config })
    }

    /// Run the application
    pub async fn run(self) -> Result<()> {
        // This is a placeholder for the real UI implementation
        info!("Starting UI with configuration: {:?}", self.config);

        // Auto-connect based on configuration and auto_connect setting
        if self.config.ui.auto_connect && self.is_config_valid() {
            info!("Configuration is valid and auto-connect is enabled, attempting connection");
            match self.connect_and_authenticate().await {
                Ok(_) => info!("Connected and authenticated successfully"),
                Err(e) => {
                    // In a real GUI app, this would prompt the user with a dialog
                    warn!("Failed to connect: {}. Would show connection dialog.", e);
                }
            }
        } else {
            if !self.config.ui.auto_connect {
                info!("Auto-connect is disabled. Waiting for user to initiate connection.");
            } else {
                // In a real GUI app, this would show a connection/config dialog
                info!("Invalid or missing configuration. Would show config dialog.");
            }
        }

        // Wait for Ctrl+C or other exit signal
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            tx.send(()).unwrap_or_default();
        });

        // Wait for Ctrl+C
        let _ = rx.await;
        info!("Shutting down UI");
        Ok(())
    }

    /// Check if the configuration is valid for auto-connection
    fn is_config_valid(&self) -> bool {
        // Check if server address is set
        if self.config.server.address.is_empty() {
            return false;
        }

        // Check if we have valid authentication information
        if let Some(auth_method) = auth::AuthMethod::from_str(&self.config.auth.method) {
            match auth_method {
                auth::AuthMethod::Password | auth::AuthMethod::Native => {
                    // These methods require a username
                    if self.config.auth.username.is_none()
                        || self.config.auth.username.as_ref().unwrap().is_empty()
                    {
                        return false;
                    }
                }
                _ => {}
            }
        } else {
            return false;
        }

        true
    }

    /// Connect to the server and authenticate
    async fn connect_and_authenticate(&self) -> Result<()> {
        // Connect to the server
        info!(
            "Connecting to server {}:{}",
            self.config.server.address, self.config.server.port
        );

        let client =
            match protocol::Client::connect(&self.config.server.address, self.config.server.port)
                .await
            {
                Ok(client) => {
                    info!("Connected to server");
                    client
                }
                Err(e) => {
                    error!("Failed to connect to server: {}", e);
                    return Err(e.into());
                }
            };

        // Get username for authentication
        let username = self.config.auth.username.clone().unwrap_or_else(|| {
            // Try to get the current OS username
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "user".to_string())
        });

        // Determine authentication method
        let auth_method = match auth::AuthMethod::from_str(&self.config.auth.method) {
            Some(method) => method,
            None => {
                warn!(
                    "Unknown authentication method: {}, falling back to password",
                    self.config.auth.method
                );
                auth::AuthMethod::Password
            }
        };

        // Authenticate
        info!("Authenticating with method: {}", auth_method);
        let auth_provider = auth::create_provider(auth_method, &username);

        match client.authenticate_with_provider(&*auth_provider).await {
            Ok(true) => {
                info!("Authentication successful");
                Ok(())
            }
            Ok(false) => {
                error!("Authentication failed");
                Err(anyhow::anyhow!("Authentication rejected"))
            }
            Err(e) => {
                error!("Authentication error: {}", e);
                Err(e)
            }
        }
    }
}
