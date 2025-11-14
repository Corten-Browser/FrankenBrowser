//! Type definitions for WebView integration

use crate::errors::{Error, Result};
use message_bus::MessageSender;

/// WebView wrapper that handles platform-specific implementations
pub struct WebViewWrapper {
    #[allow(dead_code)] // Will be used for IPC communication in GUI mode
    sender: Box<dyn MessageSender>,
    current_url: Option<String>,
    // Platform-specific WebView will be added with feature flags
}

impl WebViewWrapper {
    /// Create a new WebView instance
    pub fn new(sender: Box<dyn MessageSender>) -> Result<Self> {
        Ok(Self {
            sender,
            current_url: None,
        })
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: &str) -> Result<()> {
        // Validate URL
        if url.is_empty() {
            return Err(Error::Navigation("URL cannot be empty".to_string()));
        }

        // In headless mode, just track the URL
        self.current_url = Some(url.to_string());
        Ok(())
    }

    /// Execute JavaScript in the WebView
    pub fn execute_script(&mut self, script: &str) -> Result<String> {
        if script.is_empty() {
            return Err(Error::ScriptExecution("Script cannot be empty".to_string()));
        }

        // In headless mode, return a mock result
        // Real implementation would execute in WebView
        Ok("null".to_string())
    }

    /// Get the DOM as a string
    pub fn get_dom(&self) -> Result<String> {
        // In headless mode, return minimal DOM
        // Real implementation would get from WebView
        Ok("<html><body></body></html>".to_string())
    }

    /// Get current URL (for testing)
    pub fn current_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use message_bus::MessageBus;

    // ========================================
    // RED PHASE: Tests for WebViewWrapper::new()
    // ========================================

    #[test]
    fn test_webview_wrapper_new() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let result = WebViewWrapper::new(sender);
        assert!(result.is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_webview_wrapper_new_initializes_empty_url() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();
        assert_eq!(wrapper.current_url(), None);

        bus.shutdown().unwrap();
    }

    // ========================================
    // RED PHASE: Tests for navigate()
    // ========================================

    #[test]
    fn test_navigate_valid_url() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.navigate("https://example.com");
        assert!(result.is_ok());
        assert_eq!(wrapper.current_url(), Some("https://example.com"));

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_navigate_empty_url_fails() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.navigate("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Navigation(_)));

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_navigate_updates_current_url() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut wrapper = WebViewWrapper::new(sender).unwrap();
        wrapper.navigate("https://example.com").unwrap();
        assert_eq!(wrapper.current_url(), Some("https://example.com"));

        wrapper.navigate("https://another.com").unwrap();
        assert_eq!(wrapper.current_url(), Some("https://another.com"));

        bus.shutdown().unwrap();
    }

    // ========================================
    // RED PHASE: Tests for execute_script()
    // ========================================

    #[test]
    fn test_execute_script_returns_result() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.execute_script("console.log('test')");
        assert!(result.is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_execute_script_empty_fails() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.execute_script("");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::ScriptExecution(_)));

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_execute_script_returns_string() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let mut wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.execute_script("1 + 1").unwrap();
        assert!(!result.is_empty());

        bus.shutdown().unwrap();
    }

    // ========================================
    // RED PHASE: Tests for get_dom()
    // ========================================

    #[test]
    fn test_get_dom_returns_html() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.get_dom();
        assert!(result.is_ok());

        let dom = result.unwrap();
        assert!(dom.contains("<html"));
        assert!(dom.contains("</html>"));

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_get_dom_without_navigate() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();
        let result = wrapper.get_dom();
        assert!(result.is_ok()); // Should work even without navigation

        bus.shutdown().unwrap();
    }

    // ========================================
    // Tests for thread safety
    // ========================================

    #[test]
    fn test_webview_wrapper_send() {
        // Note: WebViewWrapper doesn't implement Send because MessageSender is !Send
        // This is expected - WebView is single-threaded and must run on main thread
        // This test documents the single-threaded nature of WebView
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();
        let _wrapper = WebViewWrapper::new(sender).unwrap();
        // If this compiles, the structure is correct
        bus.shutdown().unwrap();
    }
}
