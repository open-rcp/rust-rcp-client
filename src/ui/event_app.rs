use crate::auth;
use crate::config::ClientConfig;
use crate::protocol;
use anyhow::Result;
use log::{error, info, warn};
use std::fmt;
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

/// Application events for the event-based UI
pub enum AppEvent {
    /// Connect to server
    Connect,
    /// Connected to server successfully
    Connected(protocol::Client),
    /// Connection to server failed
    ConnectionFailed(String),
    /// Authentication succeeded
    AuthenticationSucceeded,
    /// Authentication failed
    AuthenticationFailed(String),
    /// Show connection dialog
    ShowConnectionDialog,
    /// Show authentication dialog
    ShowAuthenticationDialog,
    /// Quit application
    Quit,
}

// Manual Debug implementation since Client doesn't implement Debug
impl fmt::Debug for AppEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect => write!(f, "Connect"),
            Self::Connected(_) => write!(f, "Connected(Client)"),
            Self::ConnectionFailed(s) => write!(f, "ConnectionFailed({})", s),
            Self::AuthenticationSucceeded => write!(f, "AuthenticationSucceeded"),
            Self::AuthenticationFailed(s) => write!(f, "AuthenticationFailed({})", s),
            Self::ShowConnectionDialog => write!(f, "ShowConnectionDialog"),
            Self::ShowAuthenticationDialog => write!(f, "ShowAuthenticationDialog"),
            Self::Quit => write!(f, "Quit"),
        }
    }
}

/// Event-based application UI
pub struct EventBasedApp {
    /// Application configuration
    config: ClientConfig,
    /// Event sender
    event_tx: mpsc::Sender<AppEvent>,
    /// Event receiver
    event_rx: mpsc::Receiver<AppEvent>,
    /// RCP client
    client: Arc<Mutex<Option<protocol::Client>>>,
    /// Auto-connect flag
    auto_connect: bool,
}

impl EventBasedApp {
    /// Create a new application with the given configuration
    pub fn new(config: ClientConfig, auto_connect: bool) -> Self {
        let (event_tx, event_rx) = mpsc::channel(32);
        Self {
            config,
            event_tx,
            event_rx,
            client: Arc::new(Mutex::new(None)),
            auto_connect,
        }
    }

    /// Run the application
    pub async fn run(mut self) -> Result<()> {
        // This is a placeholder for the real UI implementation
        // In a real implementation, this would set up and run the UI
        info!("Starting UI with configuration: {:?}", self.config);

        // Auto-connect if configuration looks valid
        let should_auto_connect = self.auto_connect && self.is_config_valid();

        if should_auto_connect {
            info!("Auto-connecting on startup due to valid configuration");
            self.event_tx.send(AppEvent::Connect).await.unwrap();
        } else {
            if !self.auto_connect {
                info!("Auto-connect is disabled. Waiting for user to initiate connection.");
            } else {
                info!("No valid configuration for auto-connection");
                self.event_tx
                    .send(AppEvent::ShowConnectionDialog)
                    .await
                    .unwrap();
            }
        }

        // Wait for Ctrl+C signal
        let (ctrl_c_tx, _ctrl_c_rx) = oneshot::channel();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            tokio::signal::ctrl_c().await.unwrap();
            event_tx.send(AppEvent::Quit).await.unwrap();
            ctrl_c_tx.send(()).unwrap_or_default();
        });

        // Main event loop
        let mut quit = false;
        while !quit {
            if let Some(event) = self.event_rx.recv().await {
                quit = self.handle_event(event).await?;
            }
        }

        // Cleanup
        info!("Shutting down UI");

        Ok(())
    }

    /// Handle an application event
    async fn handle_event(&mut self, event: AppEvent) -> Result<bool> {
        match event {
            AppEvent::Connect => {
                self.connect_to_server().await?;
                Ok(false)
            }
            AppEvent::Connected(client) => {
                info!("Connected to server, attempting authentication");
                *self.client.lock().await = Some(client);
                self.authenticate().await?;
                Ok(false)
            }
            AppEvent::ConnectionFailed(error) => {
                error!("Connection failed: {}", error);
                self.event_tx.send(AppEvent::ShowConnectionDialog).await?;
                Ok(false)
            }
            AppEvent::AuthenticationSucceeded => {
                info!("Authentication succeeded");
                // In a real GUI, would update the UI to show connected state
                Ok(false)
            }
            AppEvent::AuthenticationFailed(error) => {
                error!("Authentication failed: {}", error);
                self.event_tx
                    .send(AppEvent::ShowAuthenticationDialog)
                    .await?;
                Ok(false)
            }
            AppEvent::ShowConnectionDialog => {
                // In a real GUI, would show a connection dialog
                // For now, just simulate with a log message
                info!("Would show connection dialog here. Using config values instead.");
                tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                self.event_tx.send(AppEvent::Connect).await?;
                Ok(false)
            }
            AppEvent::ShowAuthenticationDialog => {
                // In a real GUI, would show an authentication dialog
                // For now, just simulate with a log message
                info!("Would show authentication dialog here. Using config values instead.");
                Ok(false)
            }
            AppEvent::Quit => {
                info!("Quitting application");
                Ok(true)
            }
        }
    }

    /// Connect to the RCP server
    async fn connect_to_server(&self) -> Result<()> {
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();

        // Spawn a task to connect to the server
        tokio::spawn(async move {
            info!(
                "Connecting to server {}:{}",
                config.server.address, config.server.port
            );

            match protocol::Client::connect(&config.server.address, config.server.port).await {
                Ok(client) => {
                    info!("Connected to server");
                    event_tx.send(AppEvent::Connected(client)).await.unwrap();
                }
                Err(e) => {
                    error!("Failed to connect to server: {}", e);
                    event_tx
                        .send(AppEvent::ConnectionFailed(e.to_string()))
                        .await
                        .unwrap();
                }
            }
        });

        Ok(())
    }

    /// Authenticate with the server
    async fn authenticate(&self) -> Result<()> {
        let config = self.config.clone();
        let event_tx = self.event_tx.clone();
        let client_lock = self.client.clone();

        // Spawn a task to authenticate
        tokio::spawn(async move {
            // First get the client from the mutex
            let client_opt = client_lock.lock().await;
            if let Some(client) = &*client_opt {
                // Get the username
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
                        warn!(
                            "Unknown authentication method: {}, falling back to password",
                            config.auth.method
                        );
                        auth::AuthMethod::Password
                    }
                };

                info!("Authenticating with method: {}", auth_method);
                let auth_provider = auth::create_provider(auth_method, &username);

                match client.authenticate_with_provider(&*auth_provider).await {
                    Ok(true) => {
                        info!("Authentication successful");
                        event_tx
                            .send(AppEvent::AuthenticationSucceeded)
                            .await
                            .unwrap();
                    }
                    Ok(false) => {
                        error!("Authentication failed");
                        event_tx
                            .send(AppEvent::AuthenticationFailed(
                                "Authentication rejected".to_string(),
                            ))
                            .await
                            .unwrap();
                    }
                    Err(e) => {
                        error!("Authentication error: {}", e);
                        event_tx
                            .send(AppEvent::AuthenticationFailed(e.to_string()))
                            .await
                            .unwrap();
                    }
                }
            } else {
                error!("No client available for authentication");
                event_tx
                    .send(AppEvent::AuthenticationFailed(
                        "No client connection available".to_string(),
                    ))
                    .await
                    .unwrap();
            }
        });

        Ok(())
    }

    /// Check if the configuration is valid for auto-connection
    fn is_config_valid(&self) -> bool {
        // Check if server address and port are set
        if self.config.server.address.is_empty() {
            return false;
        }

        // Check if username is set for auth methods that require it
        if let Some(auth_method) = auth::AuthMethod::from_str(&self.config.auth.method) {
            match auth_method {
                auth::AuthMethod::Password | auth::AuthMethod::Native => {
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
}
