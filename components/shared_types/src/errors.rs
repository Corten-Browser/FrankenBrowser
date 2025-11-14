//! Error types for FrankenBrowser
//!
//! This module defines error types used throughout the FrankenBrowser project.

use thiserror::Error;

/// Main error type for FrankenBrowser operations
///
/// This enum represents all possible errors that can occur in the browser.
#[derive(Error, Debug)]
pub enum BrowserError {
    /// Network-related error
    #[error("Network error: {0}")]
    Network(String),

    /// Navigation-related error
    #[error("Navigation error: {0}")]
    Navigation(String),

    /// Tab not found error
    #[error("Tab not found: {0}")]
    TabNotFound(u32),

    /// Generic error from anyhow
    #[error("Error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Result type alias for FrankenBrowser operations
///
/// This is a convenience alias for `std::result::Result<T, BrowserError>`.
pub type Result<T> = std::result::Result<T, BrowserError>;

// Ensure Send + Sync for thread safety
static_assertions::assert_impl_all!(BrowserError: Send, Sync);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_error_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<BrowserError>();
    }

    #[test]
    fn test_network_error_display() {
        let error = BrowserError::Network("Connection timeout".to_string());
        assert_eq!(error.to_string(), "Network error: Connection timeout");
    }

    #[test]
    fn test_navigation_error_display() {
        let error = BrowserError::Navigation("Invalid URL".to_string());
        assert_eq!(error.to_string(), "Navigation error: Invalid URL");
    }

    #[test]
    fn test_tab_not_found_error_display() {
        let error = BrowserError::TabNotFound(42);
        assert_eq!(error.to_string(), "Tab not found: 42");
    }

    #[test]
    fn test_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("Something went wrong");
        let browser_err = BrowserError::from(anyhow_err);
        assert!(matches!(browser_err, BrowserError::Other(_)));
    }

    #[test]
    fn test_result_type() {
        let ok_result: Result<i32> = Ok(42);
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: Result<i32> = Err(BrowserError::Network("Failed".to_string()));
        assert!(err_result.is_err());
    }
}
