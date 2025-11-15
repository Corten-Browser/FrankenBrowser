//! Error types for browser_shell component

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Tab not found: {0}")]
    TabNotFound(u32),

    #[error("Invalid tab ID: {0}")]
    InvalidTabId(u32),

    #[error("No active tab")]
    NoActiveTab,

    #[error("Window error: {0}")]
    WindowError(String),

    #[error("Event loop error: {0}")]
    EventLoopError(String),

    #[error("Message sending failed: {0}")]
    MessageSendError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Runtime error: {0}")]
    RuntimeError(String),

    #[error("Initialization error: {0}")]
    Initialization(String),

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, Error>;
