//! Error types for network stack component

use thiserror::Error;

/// Errors that can occur in the network stack
#[derive(Error, Debug)]
pub enum Error {
    /// Network request failed
    #[error("Network request failed: {0}")]
    RequestFailed(String),

    /// Invalid URL
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Timeout occurred
    #[error("Request timeout")]
    Timeout,

    /// Cache error
    #[error("Cache error: {0}")]
    CacheError(String),

    /// Cookie error
    #[error("Cookie error: {0}")]
    CookieError(String),

    /// Initialization error
    #[error("Initialization error: {0}")]
    InitializationError(String),

    /// Message bus error
    #[error("Message bus error: {0}")]
    MessageBusError(String),

    /// Other error
    #[error("Other error: {0}")]
    Other(#[from] anyhow::Error),
}

/// Result type for network stack operations
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_request_failed() {
        let err = Error::RequestFailed("Connection refused".to_string());
        assert!(err.to_string().contains("Network request failed"));
        assert!(err.to_string().contains("Connection refused"));
    }

    #[test]
    fn test_error_invalid_url() {
        let err = Error::InvalidUrl("not a url".to_string());
        assert!(err.to_string().contains("Invalid URL"));
    }

    #[test]
    fn test_error_timeout() {
        let err = Error::Timeout;
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn test_error_cache_error() {
        let err = Error::CacheError("Disk full".to_string());
        assert!(err.to_string().contains("Cache error"));
    }

    #[test]
    fn test_error_cookie_error() {
        let err = Error::CookieError("Invalid cookie".to_string());
        assert!(err.to_string().contains("Cookie error"));
    }

    #[test]
    fn test_error_initialization_error() {
        let err = Error::InitializationError("Failed to create client".to_string());
        assert!(err.to_string().contains("Initialization error"));
    }

    #[test]
    fn test_error_message_bus_error() {
        let err = Error::MessageBusError("Send failed".to_string());
        assert!(err.to_string().contains("Message bus error"));
    }

    #[test]
    fn test_error_from_anyhow() {
        let anyhow_err = anyhow::anyhow!("Something went wrong");
        let err = Error::from(anyhow_err);
        assert!(matches!(err, Error::Other(_)));
    }

    #[test]
    fn test_result_type() {
        let ok_result: Result<i32> = Ok(42);
        assert_eq!(ok_result.unwrap(), 42);

        let err_result: Result<i32> = Err(Error::Timeout);
        assert!(err_result.is_err());
    }
}
