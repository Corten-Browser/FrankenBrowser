//! Error types for message bus operations

use thiserror::Error;

/// Errors that can occur in the message bus
#[derive(Error, Debug)]
pub enum Error {
    /// Bus has already been started
    #[error("Message bus already started")]
    AlreadyStarted,

    /// Bus has already been shut down
    #[error("Message bus already shut down")]
    AlreadyShutdown,

    /// Bus is not running
    #[error("Message bus not running")]
    NotRunning,

    /// Channel disconnected
    #[error("Channel disconnected")]
    ChannelDisconnected,

    /// Handler error
    #[error("Handler error: {0}")]
    HandlerError(String),

    /// Send error
    #[error("Failed to send message: {0}")]
    SendError(String),
}

/// Result type alias for message bus operations
pub type Result<T> = std::result::Result<T, Error>;
