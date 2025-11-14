//! FrankenBrowser - Main application entry point
//!
//! This is the binary entry point for the FrankenBrowser application.
//! It initializes logging, loads configuration, and runs the browser.

use cli_app::{BrowserApp, Result};
use config_manager::Config;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Load configuration
    let config = Config::load_or_default()?;

    tracing::info!("Starting FrankenBrowser...");
    tracing::debug!("Configuration loaded: {:?}", config);

    // Create and run browser application
    let app = BrowserApp::new(config)?;

    tracing::info!("Browser application initialized, starting...");

    app.run()?;

    tracing::info!("Browser application shutdown complete");

    Ok(())
}
