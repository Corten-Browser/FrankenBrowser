//! macOS WebView implementation using WKWebView
//!
//! This module provides WKWebView-specific WebView integration for macOS systems.
//! This is a stub implementation - full macOS support will be added in Phase 3.2.3.

use crate::errors::{Error, Result};
use crate::platform::WebViewConfig;

/// macOS WebView wrapper using WKWebView
///
/// This is a stub implementation. Full WKWebView integration will be added in Phase 3.2.3.
pub struct MacOSWebView {
    config: WebViewConfig,
}

impl MacOSWebView {
    /// Create a new macOS WebView (stub)
    pub fn new(config: WebViewConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Navigate to a URL (stub)
    pub fn navigate(&mut self, _url: &str) -> Result<()> {
        Err(Error::Platform(
            "macOS WebView not yet implemented".to_string(),
        ))
    }

    /// Execute JavaScript (stub)
    pub fn execute_script(&self, _script: &str) -> Result<String> {
        Err(Error::Platform(
            "macOS WebView not yet implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "macos")]
    fn test_macos_webview_stub() {
        let config = WebViewConfig::default();
        let result = MacOSWebView::new(config);
        assert!(result.is_ok());
    }
}
