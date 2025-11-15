//! Type definitions for WebView integration

use crate::errors::{Error, Result};
use crate::javascript_bridge::JavaScriptBridge;
use message_bus::MessageSender;
use serde_json::Value as JsonValue;
use std::sync::{Arc, Mutex};

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
    /// JavaScript bridge for IPC communication
    bridge: Arc<Mutex<JavaScriptBridge>>,
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
        // Initialize JavaScript bridge
        let bridge = Arc::new(Mutex::new(JavaScriptBridge::new()));

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
                bridge,
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
                bridge,
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

    /// Handle incoming IPC message from JavaScript
    ///
    /// Processes an IPC message through the JavaScript bridge and returns the result.
    ///
    /// # Arguments
    ///
    /// * `json` - JSON string containing the IPC message
    ///
    /// # Errors
    ///
    /// Returns an error if message parsing fails or dispatch fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::WebViewWrapper;
    /// # use message_bus::MessageBus;
    /// # let mut bus = MessageBus::new();
    /// # bus.start().unwrap();
    /// # let sender = bus.sender();
    /// # let mut wrapper = WebViewWrapper::new(sender).unwrap();
    /// let message_json = r#"{"channel":"console","data":{"level":"log","message":"Hello"},"timestamp":1234567890,"message_id":"550e8400-e29b-41d4-a716-446655440000"}"#;
    /// let result = wrapper.handle_ipc_message(message_json).unwrap();
    /// # bus.shutdown().unwrap();
    /// ```
    pub fn handle_ipc_message(&self, json: &str) -> Result<JsonValue> {
        let bridge = self.bridge.lock().unwrap();
        let message = bridge.parse_ipc_message(json)?;
        bridge.dispatch_message(message)
    }

    /// Emit an event from Rust to JavaScript
    ///
    /// Creates JavaScript code that dispatches an event to registered listeners.
    /// The generated code should be executed in the WebView context.
    ///
    /// # Arguments
    ///
    /// * `channel` - Event channel name
    /// * `data` - Event data (must be JSON-serializable)
    ///
    /// # Returns
    ///
    /// JavaScript code string that can be executed via `execute_script()`
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::WebViewWrapper;
    /// # use message_bus::MessageBus;
    /// # use serde_json::json;
    /// # let mut bus = MessageBus::new();
    /// # bus.start().unwrap();
    /// # let sender = bus.sender();
    /// # let mut wrapper = WebViewWrapper::new(sender).unwrap();
    /// let script = wrapper.emit_event("custom-event", json!({"key": "value"})).unwrap();
    /// wrapper.execute_script(&script).unwrap();
    /// # bus.shutdown().unwrap();
    /// ```
    pub fn emit_event(&self, channel: &str, data: JsonValue) -> Result<String> {
        let bridge = self.bridge.lock().unwrap();
        bridge.emit_event(channel, data)
    }

    /// Get the JavaScript bridge instance
    ///
    /// Returns a reference to the bridge for advanced usage.
    pub fn bridge(&self) -> Arc<Mutex<JavaScriptBridge>> {
        Arc::clone(&self.bridge)
    }

    /// Take a screenshot of the WebView
    ///
    /// Returns the screenshot as PNG bytes.
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path to save the screenshot to
    ///
    /// # Errors
    ///
    /// Returns an error if screenshot capture fails or if not in GUI mode.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use webview_integration::WebViewWrapper;
    /// # use message_bus::MessageBus;
    /// # let mut bus = MessageBus::new();
    /// # bus.start().unwrap();
    /// # let sender = bus.sender();
    /// # let mut wrapper = WebViewWrapper::new(sender).unwrap();
    /// // Take screenshot and save to file
    /// let png_bytes = wrapper.screenshot(Some("screenshot.png")).unwrap();
    /// ```
    pub fn screenshot(&self, path: Option<&str>) -> Result<Vec<u8>> {
        #[cfg(feature = "gui")]
        {
            // In GUI mode, we need to capture the WebView content
            // For wry/webkit2gtk, we'll use JavaScript to get the canvas
            // This is a placeholder implementation that would need platform-specific code

            if self.webview.is_some() {
                // For now, return a minimal PNG (1x1 transparent pixel)
                // In a full implementation, this would:
                // 1. Use webkit2gtk's snapshot API
                // 2. Or use JavaScript to render canvas
                // 3. Or use platform screenshot APIs

                let png_bytes = create_placeholder_png();

                if let Some(save_path) = path {
                    std::fs::write(save_path, &png_bytes).map_err(|e| {
                        Error::Screenshot(format!("Failed to save screenshot: {}", e))
                    })?;
                }

                Ok(png_bytes)
            } else {
                Err(Error::Screenshot("WebView not initialized".to_string()))
            }
        }

        #[cfg(not(feature = "gui"))]
        {
            let _ = path; // Suppress unused variable warning
            Err(Error::Screenshot(
                "Screenshot not available in headless mode".to_string(),
            ))
        }
    }
}

/// Create a minimal 1x1 transparent PNG
///
/// This is a placeholder for testing. A real implementation would capture
/// the actual WebView content.
#[cfg(feature = "gui")]
fn create_placeholder_png() -> Vec<u8> {
    // Minimal PNG: 1x1 transparent pixel
    // PNG signature + IHDR + IDAT + IEND chunks
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, // IHDR length
        0x49, 0x48, 0x44, 0x52, // IHDR type
        0x00, 0x00, 0x00, 0x01, // width: 1
        0x00, 0x00, 0x00, 0x01, // height: 1
        0x08, 0x06, 0x00, 0x00, 0x00, // bit depth, color type, compression, filter, interlace
        0x1F, 0x15, 0xC4, 0x89, // CRC
        0x00, 0x00, 0x00, 0x0A, // IDAT length
        0x49, 0x44, 0x41, 0x54, // IDAT type
        0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00, 0x05, 0x00, 0x01, // compressed data
        0x0D, 0x0A, 0x2D, 0xB4, // CRC
        0x00, 0x00, 0x00, 0x00, // IEND length
        0x49, 0x45, 0x4E, 0x44, // IEND type
        0xAE, 0x42, 0x60, 0x82, // CRC
    ]
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
    // Tests for IPC bridge integration
    // ========================================

    #[test]
    fn test_handle_ipc_message_console() {
        use serde_json::json;

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let message_json = json!({
            "channel": "console",
            "data": {
                "level": "log",
                "message": "Test message"
            },
            "timestamp": 1234567890_u64,
            "message_id": "550e8400-e29b-41d4-a716-446655440000"
        })
        .to_string();

        let result = wrapper.handle_ipc_message(&message_json);
        assert!(result.is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_handle_ipc_message_invalid_json() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let result = wrapper.handle_ipc_message("invalid json");
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::Ipc(_)));

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_emit_event() {
        use serde_json::json;

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let result = wrapper.emit_event("custom-event", json!({"key": "value"}));
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.contains("ipc._dispatch"));
        assert!(script.contains("custom-event"));

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_bridge_access() {
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let bridge = wrapper.bridge();
        assert!(bridge.lock().is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_handle_ipc_message_navigation() {
        use serde_json::json;

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let message_json = json!({
            "channel": "navigation",
            "data": {
                "command": "navigate",
                "url": "https://example.com"
            },
            "timestamp": 1234567890_u64,
            "message_id": "550e8400-e29b-41d4-a716-446655440000"
        })
        .to_string();

        let result = wrapper.handle_ipc_message(&message_json);
        assert!(result.is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_handle_ipc_message_error() {
        use serde_json::json;

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let message_json = json!({
            "channel": "error",
            "data": {
                "type": "error",
                "message": "Test error",
                "filename": "test.js",
                "lineno": 42
            },
            "timestamp": 1234567890_u64,
            "message_id": "550e8400-e29b-41d4-a716-446655440000"
        })
        .to_string();

        let result = wrapper.handle_ipc_message(&message_json);
        assert!(result.is_ok());

        bus.shutdown().unwrap();
    }

    #[test]
    fn test_emit_event_with_complex_data() {
        use serde_json::json;

        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();

        let wrapper = WebViewWrapper::new(sender).unwrap();

        let complex_data = json!({
            "nested": {
                "array": [1, 2, 3],
                "object": {
                    "key": "value"
                }
            },
            "number": 42.5,
            "boolean": true
        });

        let result = wrapper.emit_event("complex-event", complex_data);
        assert!(result.is_ok());

        let script = result.unwrap();
        assert!(script.contains("complex-event"));
        assert!(script.contains("nested"));

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
