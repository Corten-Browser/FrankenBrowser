//! Error types for adblock_engine component

use thiserror::Error;

/// Errors that can occur in the adblock engine
#[derive(Error, Debug)]
pub enum Error {
    /// Ad blocking is not enabled
    #[error("Ad blocking is not enabled")]
    NotEnabled,

    /// Engine already initialized
    #[error("Engine already initialized")]
    AlreadyInitialized,

    /// Engine not initialized
    #[error("Engine not initialized")]
    NotInitialized,

    /// Failed to load filter list
    #[error("Failed to load filter list: {0}")]
    FilterLoadError(String),

    /// Failed to parse filter rule
    #[error("Failed to parse filter rule: {0}")]
    FilterParseError(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other error
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for adblock_engine operations
pub type Result<T> = std::result::Result<T, Error>;
