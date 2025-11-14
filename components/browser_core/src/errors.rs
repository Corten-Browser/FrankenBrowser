//! Error types for browser_core component

use thiserror::Error;

/// Errors that can occur in the browser core
#[derive(Error, Debug)]
pub enum Error {
    /// Tab not found
    #[error("Tab {0} not found")]
    TabNotFound(u32),

    /// No history available
    #[error("No history available for tab {0}")]
    NoHistory(u32),

    /// No forward history available
    #[error("No forward history available for tab {0}")]
    NoForwardHistory(u32),

    /// No current page
    #[error("No current page for tab {0}")]
    NoCurrentPage(u32),

    /// Database error
    #[error("Database error: {0}")]
    DatabaseError(String),

    /// Network error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Other errors
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Result type alias for browser_core operations
pub type Result<T> = std::result::Result<T, Error>;

// Convert from rusqlite::Error
impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::DatabaseError(err.to_string())
    }
}

// Convert from network_stack::Error
impl From<network_stack::Error> for Error {
    fn from(err: network_stack::Error) -> Self {
        Error::NetworkError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tab_not_found_error() {
        let err = Error::TabNotFound(42);
        assert_eq!(err.to_string(), "Tab 42 not found");
    }

    #[test]
    fn test_no_history_error() {
        let err = Error::NoHistory(1);
        assert_eq!(err.to_string(), "No history available for tab 1");
    }

    #[test]
    fn test_no_forward_history_error() {
        let err = Error::NoForwardHistory(2);
        assert_eq!(err.to_string(), "No forward history available for tab 2");
    }

    #[test]
    fn test_no_current_page_error() {
        let err = Error::NoCurrentPage(3);
        assert_eq!(err.to_string(), "No current page for tab 3");
    }

    #[test]
    fn test_database_error() {
        let err = Error::DatabaseError("connection failed".to_string());
        assert!(err.to_string().contains("Database error"));
        assert!(err.to_string().contains("connection failed"));
    }

    #[test]
    fn test_network_error() {
        let err = Error::NetworkError("timeout".to_string());
        assert!(err.to_string().contains("Network error"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_invalid_url_error() {
        let err = Error::InvalidUrl("not a url".to_string());
        assert!(err.to_string().contains("Invalid URL"));
        assert!(err.to_string().contains("not a url"));
    }
}
