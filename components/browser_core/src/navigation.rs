//! Navigation module for protocol handling and error pages
//!
//! This module provides URL validation, protocol handling, and navigation state management.

use crate::errors::{Error, Result};
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{Duration, Instant};
use url::Url;

/// Navigation state
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationState {
    /// Idle state - no navigation in progress
    Idle,
    /// Loading a URL - includes start time
    Loading(Url, Instant),
    /// Successfully loaded a URL - includes duration
    Loaded(Url, Duration),
    /// Navigation error occurred
    Error(Url, NavigationError),
}

/// Navigation-specific errors
#[derive(Debug, Clone, PartialEq)]
pub enum NavigationError {
    /// Invalid URL format
    InvalidUrl(String),
    /// Unsupported protocol
    UnsupportedProtocol(String),
    /// Network error
    NetworkError(String),
    /// Timeout during navigation
    Timeout,
    /// SSL/TLS error
    SslError(String),
    /// File not found (for file:// URLs)
    FileNotFound(PathBuf),
    /// Redirect loop detected
    RedirectLoop,
}

/// Protocol types supported by the browser
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Protocol {
    /// HTTP protocol
    Http,
    /// HTTPS protocol
    Https,
    /// File protocol (local files)
    File,
    /// Data URLs (inline data)
    Data,
    /// About pages (internal pages)
    About,
    /// Unsupported protocol
    Unsupported(String),
}

/// Navigator handles URL navigation, protocol detection, and error page generation
pub struct Navigator {
    /// Current navigation state
    state: NavigationState,
    /// Redirect history for loop detection
    redirect_history: HashSet<String>,
    /// Navigation timeout duration
    #[allow(dead_code)]
    timeout_duration: Duration,
    /// Allowed protocols
    allowed_protocols: HashSet<Protocol>,
    /// Maximum number of redirects to follow
    #[allow(dead_code)]
    max_redirects: usize,
}

impl Navigator {
    /// Create a new Navigator with default settings
    ///
    /// # Returns
    ///
    /// Returns a new Navigator instance.
    ///
    /// # Examples
    ///
    /// ```
    /// use browser_core::navigation::Navigator;
    ///
    /// let navigator = Navigator::new();
    /// ```
    pub fn new() -> Self {
        let mut allowed_protocols = HashSet::new();
        allowed_protocols.insert(Protocol::Http);
        allowed_protocols.insert(Protocol::Https);
        allowed_protocols.insert(Protocol::File);
        allowed_protocols.insert(Protocol::Data);
        allowed_protocols.insert(Protocol::About);

        Self {
            state: NavigationState::Idle,
            redirect_history: HashSet::new(),
            timeout_duration: Duration::from_secs(30),
            allowed_protocols,
            max_redirects: 10,
        }
    }

    /// Get the current navigation state
    pub fn state(&self) -> &NavigationState {
        &self.state
    }

    /// Validate a URL
    ///
    /// # Arguments
    ///
    /// * `url` - URL to validate
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if valid, otherwise an error.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL has no scheme
    /// - Protocol is not supported
    pub fn validate_url(&self, url: &Url) -> Result<()> {
        let protocol = Self::determine_protocol(url);

        // Check if protocol is allowed
        if !self.allowed_protocols.contains(&protocol) {
            return Err(Error::UnsupportedProtocol(url.scheme().to_string()));
        }

        // Additional validation based on protocol
        match protocol {
            Protocol::Http | Protocol::Https => {
                // Ensure there's a host
                if url.host().is_none() {
                    return Err(Error::InvalidUrl(format!(
                        "URL missing host: {}",
                        url.as_str()
                    )));
                }
            }
            Protocol::File => {
                // File URLs should have a valid path
                if url.path().is_empty() {
                    return Err(Error::InvalidUrl(format!(
                        "File URL missing path: {}",
                        url.as_str()
                    )));
                }
            }
            Protocol::Data => {
                // Data URLs should have content after 'data:'
                if url.path().is_empty() {
                    return Err(Error::InvalidUrl(format!(
                        "Data URL missing content: {}",
                        url.as_str()
                    )));
                }
            }
            Protocol::About => {
                // About URLs are always valid
            }
            Protocol::Unsupported(scheme) => {
                return Err(Error::UnsupportedProtocol(scheme));
            }
        }

        Ok(())
    }

    /// Determine the protocol from a URL
    ///
    /// # Arguments
    ///
    /// * `url` - URL to analyze
    ///
    /// # Returns
    ///
    /// Returns the detected protocol.
    pub fn determine_protocol(url: &Url) -> Protocol {
        match url.scheme() {
            "http" => Protocol::Http,
            "https" => Protocol::Https,
            "file" => Protocol::File,
            "data" => Protocol::Data,
            "about" => Protocol::About,
            other => Protocol::Unsupported(other.to_string()),
        }
    }

    /// Navigate to a URL
    ///
    /// # Arguments
    ///
    /// * `url` - URL to navigate to
    ///
    /// # Returns
    ///
    /// Returns the navigation state.
    ///
    /// # Errors
    ///
    /// Returns an error if navigation fails.
    pub fn navigate(&mut self, url: Url) -> Result<NavigationState> {
        // Reset redirect history for new navigation
        self.redirect_history.clear();

        // Validate URL
        self.validate_url(&url)?;

        // Set state to Loading
        let start_time = Instant::now();
        self.state = NavigationState::Loading(url.clone(), start_time);

        // Determine protocol and handle accordingly
        let protocol = Self::determine_protocol(&url);

        let result = match protocol {
            Protocol::Http => self.handle_http(url.clone()),
            Protocol::Https => self.handle_https(url.clone()),
            Protocol::File => {
                let path = PathBuf::from(url.path());
                self.handle_file(path)
            }
            Protocol::Data => self.handle_data(url.path()),
            Protocol::About => {
                let page = url.path();
                self.handle_about(page)
                    .map(|html| html.into_bytes())
            }
            Protocol::Unsupported(scheme) => {
                Err(Error::UnsupportedProtocol(scheme))
            }
        };

        // Update state based on result
        match result {
            Ok(_content) => {
                let duration = start_time.elapsed();
                self.state = NavigationState::Loaded(url.clone(), duration);
                Ok(self.state.clone())
            }
            Err(e) => {
                let nav_error = self.error_to_navigation_error(&e);
                self.state = NavigationState::Error(url.clone(), nav_error.clone());
                Ok(self.state.clone())
            }
        }
    }

    /// Handle HTTP protocol
    ///
    /// # Arguments
    ///
    /// * `url` - HTTP URL to fetch
    ///
    /// # Returns
    ///
    /// Returns the content bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails.
    pub fn handle_http(&self, _url: Url) -> Result<Vec<u8>> {
        // In a real implementation, this would use the NetworkStack
        // For now, we return a placeholder response
        Ok(b"<html><body>HTTP content placeholder</body></html>".to_vec())
    }

    /// Handle HTTPS protocol
    ///
    /// # Arguments
    ///
    /// * `url` - HTTPS URL to fetch
    ///
    /// # Returns
    ///
    /// Returns the content bytes.
    ///
    /// # Errors
    ///
    /// Returns an error if the request fails or SSL validation fails.
    pub fn handle_https(&self, _url: Url) -> Result<Vec<u8>> {
        // In a real implementation, this would use the NetworkStack with TLS
        // For now, we return a placeholder response
        Ok(b"<html><body>HTTPS content placeholder</body></html>".to_vec())
    }

    /// Handle file:// protocol
    ///
    /// # Arguments
    ///
    /// * `path` - Local file path to read
    ///
    /// # Returns
    ///
    /// Returns the file contents.
    ///
    /// # Errors
    ///
    /// Returns an error if the file doesn't exist or can't be read.
    pub fn handle_file(&self, path: PathBuf) -> Result<Vec<u8>> {
        std::fs::read(&path).map_err(|_| Error::FileNotFound(path.to_string_lossy().to_string()))
    }

    /// Handle data: URLs
    ///
    /// # Arguments
    ///
    /// * `data_url` - Data URL content (after 'data:')
    ///
    /// # Returns
    ///
    /// Returns the decoded data.
    ///
    /// # Errors
    ///
    /// Returns an error if the data URL is malformed.
    pub fn handle_data(&self, data_url: &str) -> Result<Vec<u8>> {
        // Parse data URL format: data:[<mediatype>][;base64],<data>
        let parts: Vec<&str> = data_url.splitn(2, ',').collect();

        if parts.len() != 2 {
            return Err(Error::InvalidUrl(format!(
                "Invalid data URL format: {}",
                data_url
            )));
        }

        let metadata = parts[0];
        let data = parts[1];

        // Check if base64 encoded
        if metadata.contains("base64") {
            // Decode base64
            use base64::{engine::general_purpose, Engine as _};
            general_purpose::STANDARD
                .decode(data)
                .map_err(|e| Error::InvalidUrl(format!("Invalid base64 data: {}", e)))
        } else {
            // URL decode plain text data
            Ok(data.as_bytes().to_vec())
        }
    }

    /// Handle about: pages
    ///
    /// # Arguments
    ///
    /// * `page` - About page name (e.g., "blank", "version")
    ///
    /// # Returns
    ///
    /// Returns the HTML content for the about page.
    ///
    /// # Errors
    ///
    /// Returns an error if the about page is unknown.
    pub fn handle_about(&self, page: &str) -> Result<String> {
        match page {
            "blank" | "" => Ok(String::from(
                "<!DOCTYPE html><html><head><title>about:blank</title></head><body></body></html>",
            )),
            "version" => Ok(format!(
                r#"<!DOCTYPE html>
<html>
<head>
    <title>About FrankenBrowser</title>
    <style>
        body {{ font-family: Arial, sans-serif; margin: 40px; }}
        h1 {{ color: #333; }}
        .version {{ font-size: 18px; margin: 20px 0; }}
    </style>
</head>
<body>
    <h1>FrankenBrowser</h1>
    <div class="version">Version: 0.1.0</div>
    <p>A modular browser implementation in Rust.</p>
</body>
</html>"#
            )),
            _ => Err(Error::InvalidUrl(format!("Unknown about page: {}", page))),
        }
    }

    /// Generate an error page for a navigation error
    ///
    /// # Arguments
    ///
    /// * `error` - The navigation error that occurred
    ///
    /// # Returns
    ///
    /// Returns HTML content for the error page.
    pub fn generate_error_page(&self, error: &NavigationError) -> String {
        let (title, message, details) = match error {
            NavigationError::InvalidUrl(url) => (
                "Invalid URL",
                "The URL you entered is not valid.",
                format!("URL: {}", url),
            ),
            NavigationError::UnsupportedProtocol(protocol) => (
                "Unsupported Protocol",
                "This protocol is not supported by FrankenBrowser.",
                format!("Protocol: {}", protocol),
            ),
            NavigationError::NetworkError(msg) => (
                "Network Error",
                "A network error occurred while loading the page.",
                msg.clone(),
            ),
            NavigationError::Timeout => (
                "Connection Timeout",
                "The connection to the server timed out.",
                "The server took too long to respond.".to_string(),
            ),
            NavigationError::SslError(msg) => (
                "SSL Certificate Error",
                "There is a problem with this site's security certificate.",
                msg.clone(),
            ),
            NavigationError::FileNotFound(path) => (
                "File Not Found",
                "The requested file could not be found.",
                format!("Path: {}", path.display()),
            ),
            NavigationError::RedirectLoop => (
                "Redirect Loop",
                "The page is redirecting in a way that will never complete.",
                "This usually happens when the server is misconfigured.".to_string(),
            ),
        };

        format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>{title}</title>
    <style>
        body {{
            font-family: Arial, sans-serif;
            margin: 0;
            padding: 0;
            background-color: #f5f5f5;
        }}
        .container {{
            max-width: 600px;
            margin: 100px auto;
            background-color: white;
            padding: 40px;
            border-radius: 8px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        h1 {{
            color: #d32f2f;
            margin-top: 0;
        }}
        .message {{
            font-size: 16px;
            margin: 20px 0;
            color: #333;
        }}
        .details {{
            background-color: #f5f5f5;
            padding: 15px;
            border-radius: 4px;
            font-family: monospace;
            font-size: 14px;
            margin: 20px 0;
            word-break: break-all;
        }}
        .actions {{
            margin-top: 30px;
        }}
        button {{
            background-color: #1976d2;
            color: white;
            border: none;
            padding: 12px 24px;
            font-size: 14px;
            border-radius: 4px;
            cursor: pointer;
            margin-right: 10px;
        }}
        button:hover {{
            background-color: #1565c0;
        }}
        .suggestions {{
            margin-top: 30px;
            padding-top: 20px;
            border-top: 1px solid #e0e0e0;
        }}
        .suggestions h3 {{
            font-size: 16px;
            margin-top: 0;
        }}
        .suggestions ul {{
            padding-left: 20px;
        }}
        .suggestions li {{
            margin: 8px 0;
        }}
    </style>
</head>
<body>
    <div class="container">
        <h1>{title}</h1>
        <div class="message">{message}</div>
        <div class="details">{details}</div>
        <div class="actions">
            <button onclick="history.back()">Go Back</button>
            <button onclick="location.reload()">Try Again</button>
        </div>
        <div class="suggestions">
            <h3>Suggestions:</h3>
            <ul>
                <li>Check the URL for typos</li>
                <li>Check your internet connection</li>
                <li>Try reloading the page later</li>
            </ul>
        </div>
    </div>
</body>
</html>"#,
            title = title,
            message = message,
            details = details
        )
    }

    /// Follow redirects up to a maximum count
    ///
    /// # Arguments
    ///
    /// * `url` - Initial URL to follow
    /// * `max_redirects` - Maximum number of redirects to follow
    ///
    /// # Returns
    ///
    /// Returns the final URL after following redirects.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Redirect loop is detected
    /// - Maximum redirects exceeded
    pub fn follow_redirect(&mut self, url: Url, max_redirects: usize) -> Result<Url> {
        let url_str = url.as_str().to_string();

        // Check for redirect loop
        if self.redirect_history.contains(&url_str) {
            return Err(Error::RedirectLoop);
        }

        // Check max redirects
        if self.redirect_history.len() >= max_redirects {
            return Err(Error::RedirectLoop);
        }

        // Add to history
        self.redirect_history.insert(url_str);

        // In a real implementation, this would fetch the URL and check for redirect headers
        // For now, just return the URL
        Ok(url)
    }

    /// Cancel the current navigation
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if cancelled successfully.
    pub fn cancel_navigation(&mut self) -> Result<()> {
        // Set state back to Idle
        self.state = NavigationState::Idle;
        self.redirect_history.clear();
        Ok(())
    }

    /// Convert a general Error to NavigationError
    fn error_to_navigation_error(&self, error: &Error) -> NavigationError {
        match error {
            Error::InvalidUrl(msg) => NavigationError::InvalidUrl(msg.clone()),
            Error::UnsupportedProtocol(proto) => NavigationError::UnsupportedProtocol(proto.clone()),
            Error::NetworkError(msg) => NavigationError::NetworkError(msg.clone()),
            Error::Timeout => NavigationError::Timeout,
            Error::SslError(msg) => NavigationError::SslError(msg.clone()),
            Error::FileNotFound(path) => {
                NavigationError::FileNotFound(PathBuf::from(path.clone()))
            }
            Error::RedirectLoop => NavigationError::RedirectLoop,
            _ => NavigationError::NetworkError(error.to_string()),
        }
    }
}

impl Default for Navigator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // Tests for Protocol enum
    // ========================================

    #[test]
    fn test_determine_protocol_http() {
        let url = Url::parse("http://example.com").unwrap();
        assert_eq!(Navigator::determine_protocol(&url), Protocol::Http);
    }

    #[test]
    fn test_determine_protocol_https() {
        let url = Url::parse("https://example.com").unwrap();
        assert_eq!(Navigator::determine_protocol(&url), Protocol::Https);
    }

    #[test]
    fn test_determine_protocol_file() {
        let url = Url::parse("file:///path/to/file.html").unwrap();
        assert_eq!(Navigator::determine_protocol(&url), Protocol::File);
    }

    #[test]
    fn test_determine_protocol_data() {
        let url = Url::parse("data:text/html,<h1>Hello</h1>").unwrap();
        assert_eq!(Navigator::determine_protocol(&url), Protocol::Data);
    }

    #[test]
    fn test_determine_protocol_about() {
        let url = Url::parse("about:blank").unwrap();
        assert_eq!(Navigator::determine_protocol(&url), Protocol::About);
    }

    #[test]
    fn test_determine_protocol_unsupported() {
        let url = Url::parse("ftp://example.com").unwrap();
        assert_eq!(
            Navigator::determine_protocol(&url),
            Protocol::Unsupported("ftp".to_string())
        );
    }

    // ========================================
    // Tests for Navigator::new and validate_url
    // ========================================

    #[test]
    fn test_navigator_new() {
        let navigator = Navigator::new();
        assert!(matches!(navigator.state, NavigationState::Idle));
        assert_eq!(navigator.redirect_history.len(), 0);
    }

    #[test]
    fn test_validate_url_http_valid() {
        let navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();
        assert!(navigator.validate_url(&url).is_ok());
    }

    #[test]
    fn test_validate_url_https_valid() {
        let navigator = Navigator::new();
        let url = Url::parse("https://example.com").unwrap();
        assert!(navigator.validate_url(&url).is_ok());
    }

    #[test]
    fn test_validate_url_file_valid() {
        let navigator = Navigator::new();
        let url = Url::parse("file:///path/to/file.html").unwrap();
        assert!(navigator.validate_url(&url).is_ok());
    }

    #[test]
    fn test_validate_url_data_valid() {
        let navigator = Navigator::new();
        let url = Url::parse("data:text/html,<h1>Hello</h1>").unwrap();
        assert!(navigator.validate_url(&url).is_ok());
    }

    #[test]
    fn test_validate_url_about_valid() {
        let navigator = Navigator::new();
        let url = Url::parse("about:blank").unwrap();
        assert!(navigator.validate_url(&url).is_ok());
    }

    #[test]
    fn test_validate_url_unsupported_protocol() {
        let navigator = Navigator::new();
        let url = Url::parse("ftp://example.com").unwrap();
        let result = navigator.validate_url(&url);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::UnsupportedProtocol(_)));
    }

    // ========================================
    // Tests for Navigation
    // ========================================

    #[test]
    fn test_navigate_http() {
        let mut navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();
        let result = navigator.navigate(url.clone());
        assert!(result.is_ok());

        // Should be in Loaded state
        match navigator.state() {
            NavigationState::Loaded(loaded_url, _duration) => {
                assert_eq!(loaded_url.as_str(), url.as_str());
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[test]
    fn test_navigate_https() {
        let mut navigator = Navigator::new();
        let url = Url::parse("https://example.com").unwrap();
        let result = navigator.navigate(url.clone());
        assert!(result.is_ok());

        match navigator.state() {
            NavigationState::Loaded(loaded_url, _duration) => {
                assert_eq!(loaded_url.as_str(), url.as_str());
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[test]
    fn test_navigate_about_blank() {
        let mut navigator = Navigator::new();
        let url = Url::parse("about:blank").unwrap();
        let result = navigator.navigate(url.clone());
        assert!(result.is_ok());

        match navigator.state() {
            NavigationState::Loaded(loaded_url, _duration) => {
                assert_eq!(loaded_url.as_str(), url.as_str());
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[test]
    fn test_navigate_about_version() {
        let mut navigator = Navigator::new();
        let url = Url::parse("about:version").unwrap();
        let result = navigator.navigate(url.clone());
        assert!(result.is_ok());

        match navigator.state() {
            NavigationState::Loaded(loaded_url, _duration) => {
                assert_eq!(loaded_url.as_str(), url.as_str());
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[test]
    fn test_navigate_data_url() {
        let mut navigator = Navigator::new();
        let url = Url::parse("data:text/html,<h1>Hello World</h1>").unwrap();
        let result = navigator.navigate(url.clone());
        assert!(result.is_ok());

        match navigator.state() {
            NavigationState::Loaded(loaded_url, _duration) => {
                assert_eq!(loaded_url.as_str(), url.as_str());
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[test]
    fn test_navigate_unsupported_protocol() {
        let mut navigator = Navigator::new();
        let url = Url::parse("ftp://example.com").unwrap();

        // Remove ftp from allowed protocols
        navigator.allowed_protocols.remove(&Protocol::Unsupported("ftp".to_string()));

        let result = navigator.navigate(url.clone());
        // Navigation should fail for unsupported protocol
        assert!(result.is_err());
    }

    // ========================================
    // Tests for Protocol Handlers
    // ========================================

    #[test]
    fn test_handle_http() {
        let navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();
        let result = navigator.handle_http(url);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_handle_https() {
        let navigator = Navigator::new();
        let url = Url::parse("https://example.com").unwrap();
        let result = navigator.handle_https(url);
        assert!(result.is_ok());
        let content = result.unwrap();
        assert!(!content.is_empty());
    }

    #[test]
    fn test_handle_about_blank() {
        let navigator = Navigator::new();
        let result = navigator.handle_about("blank");
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("about:blank"));
    }

    #[test]
    fn test_handle_about_version() {
        let navigator = Navigator::new();
        let result = navigator.handle_about("version");
        assert!(result.is_ok());
        let html = result.unwrap();
        assert!(html.contains("FrankenBrowser"));
        assert!(html.contains("0.1.0"));
    }

    #[test]
    fn test_handle_about_unknown() {
        let navigator = Navigator::new();
        let result = navigator.handle_about("unknown");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_data_plain() {
        let navigator = Navigator::new();
        let result = navigator.handle_data("text/html,<h1>Hello</h1>");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content, b"<h1>Hello</h1>");
    }

    #[test]
    fn test_handle_data_base64() {
        let navigator = Navigator::new();
        // "Hello World" in base64 is "SGVsbG8gV29ybGQ="
        let result = navigator.handle_data("text/plain;base64,SGVsbG8gV29ybGQ=");
        assert!(result.is_ok());
        let content = result.unwrap();
        assert_eq!(content, b"Hello World");
    }

    #[test]
    fn test_handle_data_invalid() {
        let navigator = Navigator::new();
        let result = navigator.handle_data("invalid");
        assert!(result.is_err());
    }

    #[test]
    fn test_handle_file_not_found() {
        let navigator = Navigator::new();
        let path = PathBuf::from("/nonexistent/file.html");
        let result = navigator.handle_file(path);
        assert!(result.is_err());
    }

    // ========================================
    // Tests for Error Page Generation
    // ========================================

    #[test]
    fn test_generate_error_page_invalid_url() {
        let navigator = Navigator::new();
        let error = NavigationError::InvalidUrl("not a url".to_string());
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("Invalid URL"));
        assert!(html.contains("not a url"));
    }

    #[test]
    fn test_generate_error_page_unsupported_protocol() {
        let navigator = Navigator::new();
        let error = NavigationError::UnsupportedProtocol("ftp".to_string());
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("Unsupported Protocol"));
        assert!(html.contains("ftp"));
    }

    #[test]
    fn test_generate_error_page_network_error() {
        let navigator = Navigator::new();
        let error = NavigationError::NetworkError("Connection refused".to_string());
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("Network Error"));
        assert!(html.contains("Connection refused"));
    }

    #[test]
    fn test_generate_error_page_timeout() {
        let navigator = Navigator::new();
        let error = NavigationError::Timeout;
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("Connection Timeout"));
        assert!(html.contains("timed out"));
    }

    #[test]
    fn test_generate_error_page_ssl_error() {
        let navigator = Navigator::new();
        let error = NavigationError::SslError("Certificate expired".to_string());
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("SSL Certificate Error"));
        assert!(html.contains("Certificate expired"));
    }

    #[test]
    fn test_generate_error_page_file_not_found() {
        let navigator = Navigator::new();
        let path = PathBuf::from("/path/to/file.html");
        let error = NavigationError::FileNotFound(path);
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("File Not Found"));
        assert!(html.contains("/path/to/file.html"));
    }

    #[test]
    fn test_generate_error_page_redirect_loop() {
        let navigator = Navigator::new();
        let error = NavigationError::RedirectLoop;
        let html = navigator.generate_error_page(&error);
        assert!(html.contains("Redirect Loop"));
        assert!(html.contains("redirecting"));
    }

    // ========================================
    // Tests for Redirect Handling
    // ========================================

    #[test]
    fn test_follow_redirect_first() {
        let mut navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();
        let result = navigator.follow_redirect(url.clone(), 10);
        assert!(result.is_ok());
        assert_eq!(navigator.redirect_history.len(), 1);
    }

    #[test]
    fn test_follow_redirect_loop_detection() {
        let mut navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();

        // First redirect should succeed
        let result1 = navigator.follow_redirect(url.clone(), 10);
        assert!(result1.is_ok());

        // Second redirect to same URL should fail (loop detected)
        let result2 = navigator.follow_redirect(url, 10);
        assert!(result2.is_err());
        assert!(matches!(result2.unwrap_err(), Error::RedirectLoop));
    }

    #[test]
    fn test_follow_redirect_max_exceeded() {
        let mut navigator = Navigator::new();

        // Add URLs until we exceed max_redirects
        for i in 0..11 {
            let url = Url::parse(&format!("http://example{}.com", i)).unwrap();
            let result = navigator.follow_redirect(url, 10);

            if i < 10 {
                assert!(result.is_ok());
            } else {
                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), Error::RedirectLoop));
            }
        }
    }

    // ========================================
    // Tests for Navigation Cancellation
    // ========================================

    #[test]
    fn test_cancel_navigation() {
        let mut navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();

        // Start navigation
        navigator.navigate(url).unwrap();

        // Cancel
        let result = navigator.cancel_navigation();
        assert!(result.is_ok());
        assert!(matches!(navigator.state(), NavigationState::Idle));
        assert_eq!(navigator.redirect_history.len(), 0);
    }

    #[test]
    fn test_cancel_navigation_clears_redirect_history() {
        let mut navigator = Navigator::new();
        let url = Url::parse("http://example.com").unwrap();

        // Add some redirect history
        navigator.follow_redirect(url, 10).unwrap();
        assert_eq!(navigator.redirect_history.len(), 1);

        // Cancel should clear history
        navigator.cancel_navigation().unwrap();
        assert_eq!(navigator.redirect_history.len(), 0);
    }

    // ========================================
    // Tests for NavigationState
    // ========================================

    #[test]
    fn test_navigation_state_idle() {
        let state = NavigationState::Idle;
        assert!(matches!(state, NavigationState::Idle));
    }

    #[test]
    fn test_navigation_state_loading() {
        let url = Url::parse("http://example.com").unwrap();
        let start = Instant::now();
        let state = NavigationState::Loading(url.clone(), start);

        match state {
            NavigationState::Loading(loaded_url, _time) => {
                assert_eq!(loaded_url, url);
            }
            _ => panic!("Expected Loading state"),
        }
    }

    #[test]
    fn test_navigation_state_loaded() {
        let url = Url::parse("http://example.com").unwrap();
        let duration = Duration::from_secs(1);
        let state = NavigationState::Loaded(url.clone(), duration);

        match state {
            NavigationState::Loaded(loaded_url, dur) => {
                assert_eq!(loaded_url, url);
                assert_eq!(dur, duration);
            }
            _ => panic!("Expected Loaded state"),
        }
    }

    #[test]
    fn test_navigation_state_error() {
        let url = Url::parse("http://example.com").unwrap();
        let error = NavigationError::Timeout;
        let state = NavigationState::Error(url.clone(), error.clone());

        match state {
            NavigationState::Error(error_url, err) => {
                assert_eq!(error_url, url);
                assert_eq!(err, error);
            }
            _ => panic!("Expected Error state"),
        }
    }

    // ========================================
    // Tests for NavigationError
    // ========================================

    #[test]
    fn test_navigation_error_equality() {
        let err1 = NavigationError::Timeout;
        let err2 = NavigationError::Timeout;
        assert_eq!(err1, err2);
    }

    #[test]
    fn test_navigation_error_clone() {
        let err = NavigationError::NetworkError("test".to_string());
        let cloned = err.clone();
        assert_eq!(err, cloned);
    }

    // ========================================
    // Tests for Default trait
    // ========================================

    #[test]
    fn test_navigator_default() {
        let navigator = Navigator::default();
        assert!(matches!(navigator.state, NavigationState::Idle));
    }
}
