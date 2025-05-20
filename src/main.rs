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
    
    /// Use simple text-based interface instead of GUI
    #[clap(long, action)]
    no_gui: bool,
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

    // Disable auto-connect on startup
    config.ui.auto_connect = false;
    
    // Create a Tokio runtime handle for the GUI
    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?;
    let rt_handle = rt.handle().clone();

    // Create a shutdown channel for the GUI
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    // Use default options - we'll configure the font size in the app itself
    let options = eframe::NativeOptions::default();
    
    // It's important that eframe::run_native is called on the main thread.
    // The RcpClientApp::new method will spawn its own async tasks onto the provided rt_handle.
    
    // We need to run the eframe GUI on the main thread and the tokio runtime on a separate thread.
    // However, RcpClientApp::new expects to be called from within a context where it can spawn tokio tasks.
    // The simplest way is to ensure that eframe::run_native is the last call in main for the GUI path.
    // The RcpClientApp itself will use the rt_handle to spawn its async logic.

    // The main function is already a tokio::main, so we have a runtime.
    // We can pass its handle directly.
    
    let app_config = config.clone(); // Clone config for the app

    // Spawn a task to gracefully shutdown the runtime when the GUI exits
    let _rt_handle_shutdown = rt.handle().clone();
    tokio::spawn(async move {
        let _ = shutdown_rx.await;
        info!("GUI shutdown signal received, Tokio runtime will be shutdown if no other tasks are pending.");
        // Dropping the runtime handle or the runtime itself if it was owned here would shut it down.
        // Since rt is local to this block, it will be dropped when main exits or this block finishes.
        // For a more explicit shutdown, one might use rt.shutdown_background() or rt.shutdown_timeout().
    });

    eframe::run_native(
        "Rust RCP Client",
        options,
        Box::new(move |cc| {
            // Create RcpClientApp within the eframe closure
            // Pass the existing rt_handle from the main tokio runtime
            Box::new(crate::ui::gui::RcpClientApp::new(cc, app_config, rt_handle, shutdown_tx))
        }),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {}", e))?;

    Ok(())
}
