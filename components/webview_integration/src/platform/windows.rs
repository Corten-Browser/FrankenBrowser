//! Windows WebView implementation using WebView2
//!
//! This module provides WebView2-specific WebView integration for Windows systems.
//! This is a stub implementation - full Windows support will be added in Phase 3.2.2.

use crate::errors::{Error, Result};
use crate::platform::WebViewConfig;

/// Windows WebView wrapper using WebView2
///
/// This is a stub implementation. Full WebView2 integration will be added in Phase 3.2.2.
pub struct WindowsWebView {
    config: WebViewConfig,
}

impl WindowsWebView {
    /// Create a new Windows WebView (stub)
    pub fn new(config: WebViewConfig) -> Result<Self> {
        config.validate()?;
        Ok(Self { config })
    }

    /// Navigate to a URL (stub)
    pub fn navigate(&mut self, _url: &str) -> Result<()> {
        Err(Error::Platform(
            "Windows WebView not yet implemented".to_string(),
        ))
    }

    /// Execute JavaScript (stub)
    pub fn execute_script(&self, _script: &str) -> Result<String> {
        Err(Error::Platform(
            "Windows WebView not yet implemented".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[cfg(target_os = "windows")]
    fn test_windows_webview_stub() {
        let config = WebViewConfig::default();
        let result = WindowsWebView::new(config);
        assert!(result.is_ok());
    }
}
