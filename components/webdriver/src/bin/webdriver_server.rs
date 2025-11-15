//! WebDriver server binary
//!
//! Starts the WebDriver HTTP server on port 4444 for testing

use webdriver::server::start_server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    println!("Starting FrankenBrowser WebDriver server on port 4444...");
    println!("Press Ctrl+C to stop");

    // Start server
    start_server(4444).await?;

    Ok(())
}
