use anyhow::Result;
use clap::Parser;
use log::{info, LevelFilter};
use std::path::PathBuf;

mod auth;
mod config;
mod protocol;
mod resources;
mod ui;

/// RCP Client - Remote Control Protocol Client Application
#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to the configuration file
    #[clap(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Server address to connect to
    #[clap(short, long)]
    server: Option<String>,

    /// Verbose mode (repeat for more verbosity)
    #[clap(short, long, action = clap::ArgAction::Count)]
    verbose: u8,

    /// Username for authentication
    #[clap(short, long)]
    username: Option<String>,

    /// Authentication method (password, psk, native)
    #[clap(long, value_parser = ["password", "psk", "native"])]
    auth_method: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Parse command line arguments
    let args = Args::parse();

    // Configure logging based on verbosity level
    let log_level = match args.verbose {
        0 => LevelFilter::Info,
        1 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    env_logger::Builder::new()
        .filter_level(log_level)
        .format_timestamp_millis()
        .init();

    info!("Starting RCP client v{}", env!("CARGO_PKG_VERSION"));

    // Load configuration
    let config_path = args.config.unwrap_or_else(|| {
        dirs::config_dir()
            .expect("Could not find config directory")
            .join("rcp_client")
            .join("config.toml")
    });

    let mut config = config::load_config(&config_path).await?;
    info!("Configuration loaded from {:?}", config_path);

    // Override configuration with command-line arguments
    if let Some(server) = args.server {
        config.server.address = server;
    }

    if let Some(username) = args.username {
        config.auth.username = Some(username);
    }

    if let Some(auth_method) = args.auth_method {
        config.auth.method = auth_method;
    }

    // Connect to the server
    info!(
        "Connecting to server {}:{}",
        config.server.address, config.server.port
    );
    let client = match protocol::Client::connect(&config.server.address, config.server.port).await {
        Ok(client) => {
            info!("Connected to server");
            client
        }
        Err(e) => {
            log::error!("Failed to connect to server: {}", e);
            return Err(e.into());
        }
    };

    // Authenticate with the server
    let username = config.auth.username.clone().unwrap_or_else(|| {
        // Try to get the current OS username
        std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_else(|_| "user".to_string())
    });

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

    info!("Authenticating with method: {}", auth_method);
    let auth_provider = auth::create_provider(auth_method, &username);

    match client.authenticate_with_provider(&*auth_provider).await {
        Ok(true) => {
            info!("Authentication successful");
        }
        Ok(false) => {
            log::error!("Authentication failed");
            return Err(anyhow::anyhow!("Authentication failed"));
        }
        Err(e) => {
            log::error!("Authentication error: {}", e);
            return Err(e);
        }
    }

    // Initialize the UI
    let app = ui::App::new(config)?;

    // Run the application
    app.run().await?;

    Ok(())
}
