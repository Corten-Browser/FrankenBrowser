//! Error types for WebView integration

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Failed to initialize WebView: {0}")]
    Initialization(String),

    #[error("Navigation error: {0}")]
    Navigation(String),

    #[error("Script execution error: {0}")]
    ScriptExecution(String),

    #[error("IPC error: {0}")]
    Ipc(String),

    #[error("Platform-specific error: {0}")]
    Platform(String),

    #[error("Screenshot error: {0}")]
    Screenshot(String),

    #[error("Not implemented yet")]
    NotImplemented,
}

pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // RED PHASE: Tests for Error types
    // ========================================

    #[test]
    fn test_webview_error_initialization() {
        let error = Error::Initialization("Failed to create webview".to_string());
        assert!(matches!(error, Error::Initialization(_)));
    }

    #[test]
    fn test_webview_error_navigation() {
        let error = Error::Navigation("Invalid URL".to_string());
        assert!(matches!(error, Error::Navigation(_)));
    }

    #[test]
    fn test_webview_error_script_execution() {
        let error = Error::ScriptExecution("JavaScript error".to_string());
        assert!(matches!(error, Error::ScriptExecution(_)));
    }

    #[test]
    fn test_webview_error_ipc() {
        let error = Error::Ipc("IPC channel closed".to_string());
        assert!(matches!(error, Error::Ipc(_)));
    }

    #[test]
    fn test_webview_error_platform() {
        let error = Error::Platform("Platform-specific error".to_string());
        assert!(matches!(error, Error::Platform(_)));
    }

    #[test]
    fn test_webview_error_display() {
        let error = Error::Navigation("Test error".to_string());
        let display = format!("{}", error);
        assert!(display.contains("Test error"));
    }

    #[test]
    fn test_webview_error_debug() {
        let error = Error::Navigation("Test error".to_string());
        let debug = format!("{:?}", error);
        assert!(debug.contains("Navigation"));
    }

    #[test]
    fn test_webview_error_send_sync() {
        // Error must implement Send + Sync for thread safety
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Error>();
    }

    #[test]
    fn test_result_type_ok() {
        let result: Result<i32> = Ok(42);
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_result_type_err() {
        let result: Result<i32> = Err(Error::Navigation("Failed".to_string()));
        assert!(result.is_err());
    }
}
