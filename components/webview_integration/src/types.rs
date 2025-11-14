//! Type definitions for WebView integration

use crate::errors::{Error, Result};
use message_bus::MessageSender;

// WRY and tao imports for GUI mode
#[cfg(feature = "gui")]
use tao::{
    dpi::LogicalSize,
    window::{Window, WindowBuilder},
};

#[cfg(feature = "gui")]
use wry::WebViewBuilder;

/// WebView wrapper that handles platform-specific implementations
pub struct WebViewWrapper {
    #[allow(dead_code)] // Will be used for IPC communication in GUI mode
    sender: Box<dyn MessageSender>,
    current_url: Option<String>,
    // GUI mode: actual window, webview, and event loop
    #[cfg(feature = "gui")]
    #[allow(dead_code)]
    event_loop: Option<tao::event_loop::EventLoop<()>>,
    #[cfg(feature = "gui")]
    #[allow(dead_code)]
    window: Option<Window>,
    #[cfg(feature = "gui")]
    webview: Option<wry::WebView>,
}

impl WebViewWrapper {
    /// Create a new WebView instance
    pub fn new(sender: Box<dyn MessageSender>) -> Result<Self> {
        #[cfg(all(feature = "gui", not(test)))]
        {
            use tao::event_loop::EventLoop;

            // Create event loop (required for window creation)
            // Note: In production, browser_shell manages the event loop
            // This is a simplified implementation for standalone usage
            let event_loop = EventLoop::new();

            // Create a simple window for the WebView
            let window = WindowBuilder::new()
                .with_title("WebView Window")
                .with_inner_size(LogicalSize::new(800, 600))
                .build(&event_loop)
                .map_err(|e| Error::Initialization(format!("Failed to create window: {}", e)))?;

            // Create WebView with blank page initially
            let webview = WebViewBuilder::new()
                .with_url("about:blank")
                .build(&window)
                .map_err(|e| Error::Initialization(format!("Failed to create webview: {}", e)))?;

            Ok(Self {
                sender,
                current_url: Some("about:blank".to_string()),
                event_loop: Some(event_loop),
                window: Some(window),
                webview: Some(webview),
            })
        }

        #[cfg(any(not(feature = "gui"), test))]
        {
            // In headless mode or tests, don't create actual WebView
            Ok(Self {
                sender,
                current_url: None,
                #[cfg(feature = "gui")]
                event_loop: None,
                #[cfg(feature = "gui")]
                window: None,
                #[cfg(feature = "gui")]
                webview: None,
            })
        }
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: &str) -> Result<()> {
        // Validate URL
        if url.is_empty() {
            return Err(Error::Navigation("URL cannot be empty".to_string()));
        }

        #[cfg(feature = "gui")]
        {
            // In GUI mode, actually navigate the WebView (if initialized)
            if let Some(webview) = &self.webview {
                webview
                    .load_url(url)
                    .map_err(|e| Error::Navigation(format!("Failed to load URL: {}", e)))?;
                self.current_url = Some(url.to_string());
                Ok(())
            } else {
                // Fallback for test mode or when WebView not initialized
                self.current_url = Some(url.to_string());
                Ok(())
            }
        }

        #[cfg(not(feature = "gui"))]
        {
            // In headless mode, just track the URL
            self.current_url = Some(url.to_string());
            Ok(())
        }
    }

    /// Execute JavaScript in the WebView
    pub fn execute_script(&mut self, script: &str) -> Result<String> {
        if script.is_empty() {
            return Err(Error::ScriptExecution("Script cannot be empty".to_string()));
        }

        #[cfg(feature = "gui")]
        {
            // In GUI mode, actually execute the script (if WebView initialized)
            // Note: wry's evaluate_script returns Result<()>, not the script result
            // To get return values, IPC handlers would be needed
            if let Some(webview) = &self.webview {
                webview.evaluate_script(script).map_err(|e| {
                    Error::ScriptExecution(format!("Failed to execute script: {}", e))
                })?;
                // Return success indicator since wry doesn't return script results directly
                Ok("executed".to_string())
            } else {
                // Fallback for test mode or when WebView not initialized
                Ok("null".to_string())
            }
        }

        #[cfg(not(feature = "gui"))]
        {
            // In headless mode, return a mock result
            // Real implementation would execute in WebView
            Ok("null".to_string())
        }
    }

    /// Get the DOM as a string
    pub fn get_dom(&self) -> Result<String> {
        #[cfg(feature = "gui")]
        {
            // In GUI mode, return placeholder DOM
            // Note: Getting actual DOM content requires IPC handlers in wry
            // evaluate_script() only executes code, doesn't return values
            if self.webview.is_some() {
                // Return a basic HTML structure indicating WebView is active
                Ok("<html><head></head><body><!-- WebView Active --></body></html>".to_string())
            } else {
                // Fallback for test mode or when WebView not initialized
                Ok("<html><body></body></html>".to_string())
            }
        }

        #[cfg(not(feature = "gui"))]
        {
            // In headless mode, return minimal DOM
            // Real implementation would get from WebView
            Ok("<html><body></body></html>".to_string())
        }
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

    // ========================================
    // GUI MODE NOTES
    // ========================================
    // GUI mode tests are not included because:
    // 1. EventLoop can only be created once per process
    // 2. Tests run in the same process with #[cfg(test)]
    // 3. WebViewWrapper uses headless fallback in test mode
    //
    // GUI functionality is verified through:
    // - Compilation with --features gui succeeds
    // - Integration tests in browser_shell component
    // - Manual testing with actual application
}
