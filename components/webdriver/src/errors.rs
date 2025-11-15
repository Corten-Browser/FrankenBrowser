//! Error types for WebDriver protocol implementation

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("Session not found: {0}")]
    SessionNotFound(String),

    #[error("Invalid session: {0}")]
    InvalidSession(String),

    #[error("Invalid argument: {0}")]
    InvalidArgument(String),

    #[error("No such element: {0}")]
    NoSuchElement(String),

    #[error("No such window: {0}")]
    NoSuchWindow(String),

    #[error("JavaScript error: {0}")]
    JavaScriptError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("Unable to capture screenshot: {0}")]
    ScreenshotError(String),

    #[error("Navigation error: {0}")]
    NavigationError(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Not implemented: {0}")]
    NotImplemented(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// WebDriver error response format per W3C specification
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct WebDriverErrorResponse {
    pub value: WebDriverError,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct WebDriverError {
    pub error: String,
    pub message: String,
    pub stacktrace: String,
}

impl From<Error> for WebDriverErrorResponse {
    fn from(err: Error) -> Self {
        let error_code = match &err {
            Error::SessionNotFound(_) => "invalid session id",
            Error::InvalidSession(_) => "invalid session id",
            Error::InvalidArgument(_) => "invalid argument",
            Error::NoSuchElement(_) => "no such element",
            Error::NoSuchWindow(_) => "no such window",
            Error::JavaScriptError(_) => "javascript error",
            Error::Timeout(_) => "timeout",
            Error::ScreenshotError(_) => "unable to capture screenshot",
            Error::NavigationError(_) => "unknown error",
            Error::ServerError(_) => "unknown error",
            Error::NotImplemented(_) => "unsupported operation",
        };

        WebDriverErrorResponse {
            value: WebDriverError {
                error: error_code.to_string(),
                message: err.to_string(),
                stacktrace: String::new(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let error = Error::SessionNotFound("test-session".to_string());
        assert!(error.to_string().contains("test-session"));
    }

    #[test]
    fn test_error_to_webdriver_response() {
        let error = Error::NoSuchElement("button#submit".to_string());
        let response: WebDriverErrorResponse = error.into();
        assert_eq!(response.value.error, "no such element");
        assert!(response.value.message.contains("button#submit"));
    }

    #[test]
    fn test_invalid_argument_error() {
        let error = Error::InvalidArgument("Invalid selector".to_string());
        let response: WebDriverErrorResponse = error.into();
        assert_eq!(response.value.error, "invalid argument");
    }
}
