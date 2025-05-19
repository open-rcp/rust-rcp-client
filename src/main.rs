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

    /// Connect in background (don't force connection on startup)
    #[clap(long, action)]
    background_connect: bool,

    /// Use event-based UI implementation
    #[clap(long, action)]
    event_based: bool,
    
    /// Use graphical user interface
    #[clap(long, action)]
    gui: bool,
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

    // Try to load configuration, but use defaults if it fails
    let mut config = match config::load_config(&config_path).await {
        Ok(config) => {
            info!("Configuration loaded from {:?}", config_path);
            config
        }
        Err(e) => {
            log::warn!("Failed to load configuration: {}", e);
            log::info!("Using default configuration");
            config::ClientConfig::default()
        }
    };

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

    // Pass the background_connect flag to the UI
    config.ui.auto_connect = !args.background_connect;

    // Decide which UI implementation to use based on the command-line flags
    if args.gui {
        info!("Using graphical UI implementation");
        // Initialize the GUI
        ui::run_gui(config.clone(), !args.background_connect)?;
    } else if args.event_based {
        info!("Using event-based UI implementation");
        // Initialize the event-based UI
        let app = ui::EventBasedApp::new(config.clone(), !args.background_connect);
        app.run().await?;
    } else {
        info!("Using simple UI implementation");
        // Initialize the simple UI
        let app = ui::App::new(config.clone())?;
        app.run().await?;
    }

    Ok(())
}
