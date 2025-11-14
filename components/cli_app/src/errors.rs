//! Error types for the CLI application

use thiserror::Error;

/// Main error type for the CLI application
#[derive(Error, Debug)]
pub enum Error {
    /// Error from browser shell component
    #[error("Browser shell error: {0}")]
    BrowserShell(#[from] browser_shell::Error),

    /// Error from config manager component
    #[error("Configuration error: {0}")]
    Config(#[from] shared_types::BrowserError),

    /// Error from message bus component
    #[error("Message bus error: {0}")]
    MessageBus(#[from] message_bus::Error),

    /// Error from network stack component
    #[error("Network stack error: {0}")]
    Network(#[from] network_stack::Error),

    /// Error from adblock engine component
    #[error("AdBlock engine error: {0}")]
    AdBlock(#[from] adblock_engine::Error),

    /// Error from browser core component
    #[error("Browser core error: {0}")]
    BrowserCore(#[from] browser_core::Error),

    /// Generic error
    #[error("Application error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for CLI application
pub type Result<T> = std::result::Result<T, Error>;
