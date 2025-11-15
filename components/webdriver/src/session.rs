//! WebDriver session management

use crate::element::ElementCache;
use crate::errors::{Error, Result};
use browser_core::BrowserEngine;
use webview_integration::WebViewWrapper;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicU32, Ordering};
use uuid::Uuid;

/// Represents a window handle with associated metadata
#[derive(Debug, Clone)]
pub struct WindowHandle {
    /// Unique window handle UUID
    pub handle: String,
    /// Browser tab ID (if using browser_shell)
    pub tab_id: u32,
    /// Window title
    pub title: Option<String>,
    /// Current URL
    pub url: Option<String>,
}

impl WindowHandle {
    /// Create a new window handle
    pub fn new(handle: String, tab_id: u32) -> Self {
        Self {
            handle,
            tab_id,
            title: None,
            url: None,
        }
    }
}

/// WebDriver session representing a browser instance
pub struct Session {
    pub id: String,
    pub capabilities: Capabilities,
    pub current_url: Option<String>,
    pub window_handle: String,
    /// Browser engine for navigation and history (headless compatible)
    pub browser_engine: Option<Arc<Mutex<BrowserEngine>>>,
    /// WebView wrapper for rendering and script execution (headless compatible)
    pub webview: Option<Arc<Mutex<WebViewWrapper>>>,
    /// Element cache for managing element references
    pub element_cache: ElementCache,
    /// Map of window handles (UUID) to WindowHandle metadata
    window_handles: HashMap<String, WindowHandle>,
    /// Currently active window handle
    current_window: Option<String>,
    /// Counter for generating tab IDs
    next_tab_id: AtomicU32,
}

impl Session {
    /// Create a new session with random ID
    pub fn new(capabilities: Capabilities) -> Self {
        let id = Uuid::new_v4().to_string();
        let window_handle = Uuid::new_v4().to_string();

        // Create initial window handle
        let mut window_handles = HashMap::new();
        let initial_handle = WindowHandle::new(window_handle.clone(), 1);
        window_handles.insert(window_handle.clone(), initial_handle);

        Self {
            id,
            capabilities,
            current_url: None,
            window_handle: window_handle.clone(),
            browser_engine: None,
            webview: None,
            element_cache: ElementCache::new(),
            window_handles,
            current_window: Some(window_handle),
            next_tab_id: AtomicU32::new(2), // Start at 2 since we used 1 for initial window
        }
    }

    /// Create a new session with browser components
    pub fn new_with_browser(
        capabilities: Capabilities,
        browser_engine: BrowserEngine,
        webview: WebViewWrapper,
    ) -> Self {
        let id = Uuid::new_v4().to_string();
        let window_handle = Uuid::new_v4().to_string();

        // Create initial window handle
        let mut window_handles = HashMap::new();
        let initial_handle = WindowHandle::new(window_handle.clone(), 1);
        window_handles.insert(window_handle.clone(), initial_handle);

        Self {
            id,
            capabilities,
            current_url: None,
            window_handle: window_handle.clone(),
            browser_engine: Some(Arc::new(Mutex::new(browser_engine))),
            webview: Some(Arc::new(Mutex::new(webview))),
            element_cache: ElementCache::new(),
            window_handles,
            current_window: Some(window_handle),
            next_tab_id: AtomicU32::new(2),
        }
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: String) -> Result<()> {
        // Validate URL
        url::Url::parse(&url)
            .map_err(|e| Error::InvalidArgument(format!("Invalid URL: {}", e)))?;

        // Invalidate element cache on navigation (prevents stale element errors)
        self.element_cache.invalidate_cache();

        // If webview is available, navigate it
        if let Some(webview) = &self.webview {
            let mut webview = webview.lock().unwrap();
            webview.navigate(&url)
                .map_err(|e| Error::NavigationError(format!("WebView navigation failed: {}", e)))?;
        }

        // Update current URL
        self.current_url = Some(url);
        Ok(())
    }

    /// Get current URL
    pub fn get_url(&self) -> Option<String> {
        // If webview is available, get URL from it
        if let Some(webview) = &self.webview {
            let webview = webview.lock().unwrap();
            if let Some(url) = webview.current_url() {
                return Some(url.to_string());
            }
        }
        // Fallback to stored URL
        self.current_url.clone()
    }

    /// Execute JavaScript in the session's webview
    pub fn execute_script(&self, script: &str) -> Result<String> {
        if let Some(webview) = &self.webview {
            let mut webview = webview.lock().unwrap();
            webview.execute_script(script)
                .map_err(|e| Error::JavaScriptError(format!("Script execution failed: {}", e)))
        } else {
            // No webview available, return null
            Ok("null".to_string())
        }
    }

    /// Take screenshot of the session's webview
    pub fn screenshot(&self) -> Result<Vec<u8>> {
        if let Some(webview) = &self.webview {
            let webview = webview.lock().unwrap();
            webview.screenshot(None)
                .map_err(|e| Error::ScreenshotError(format!("Screenshot failed: {}", e)))
        } else {
            // No webview, return minimal PNG
            Ok(create_placeholder_screenshot())
        }
    }

    /// Get DOM from the session's webview
    pub fn get_dom(&self) -> Result<String> {
        if let Some(webview) = &self.webview {
            let webview = webview.lock().unwrap();
            webview.get_dom()
                .map_err(|e| Error::JavaScriptError(format!("Get DOM failed: {}", e)))
        } else {
            Ok("<html><body></body></html>".to_string())
        }
    }

    // =================================================================
    // Window Management Methods
    // =================================================================

    /// Create a new window handle
    ///
    /// # Arguments
    ///
    /// * `tab_id` - Optional tab ID (if None, generates new one)
    ///
    /// # Returns
    ///
    /// The UUID handle string for the new window
    pub fn new_window_handle(&mut self, tab_id: Option<u32>) -> String {
        let handle = Uuid::new_v4().to_string();
        let tab_id = tab_id.unwrap_or_else(|| {
            self.next_tab_id.fetch_add(1, Ordering::SeqCst)
        });

        let window_handle = WindowHandle::new(handle.clone(), tab_id);
        self.window_handles.insert(handle.clone(), window_handle);

        handle
    }

    /// Get a window handle by UUID
    ///
    /// # Arguments
    ///
    /// * `handle` - The window handle UUID to look up
    ///
    /// # Returns
    ///
    /// Reference to the WindowHandle if found
    pub fn get_window_handle(&self, handle: &str) -> Option<&WindowHandle> {
        self.window_handles.get(handle)
    }

    /// Get all window handles as UUIDs
    ///
    /// # Returns
    ///
    /// Vector of all window handle UUIDs
    pub fn get_all_window_handles(&self) -> Vec<String> {
        self.window_handles.keys().cloned().collect()
    }

    /// Switch to a different window
    ///
    /// # Arguments
    ///
    /// * `handle` - The window handle UUID to switch to
    ///
    /// # Errors
    ///
    /// Returns NoSuchWindow if the handle doesn't exist
    pub fn switch_to_window(&mut self, handle: &str) -> Result<()> {
        if !self.window_handles.contains_key(handle) {
            return Err(Error::NoSuchWindow(format!(
                "Window handle not found: {}",
                handle
            )));
        }

        self.current_window = Some(handle.to_string());
        self.window_handle = handle.to_string(); // Update legacy field for compatibility

        Ok(())
    }

    /// Close a window by handle
    ///
    /// # Arguments
    ///
    /// * `handle` - The window handle UUID to close
    ///
    /// # Returns
    ///
    /// Vector of remaining window handles
    ///
    /// # Errors
    ///
    /// Returns NoSuchWindow if the handle doesn't exist
    pub fn close_window(&mut self, handle: &str) -> Result<Vec<String>> {
        if !self.window_handles.contains_key(handle) {
            return Err(Error::NoSuchWindow(format!(
                "Window handle not found: {}",
                handle
            )));
        }

        self.window_handles.remove(handle);

        // If we closed the current window, switch to another one
        if self.current_window.as_deref() == Some(handle) {
            self.current_window = self.window_handles.keys().next().cloned();
            if let Some(new_handle) = &self.current_window {
                self.window_handle = new_handle.clone();
            }
        }

        Ok(self.get_all_window_handles())
    }

    /// Get current window handle
    ///
    /// # Returns
    ///
    /// Current window handle UUID
    pub fn get_current_window(&self) -> Option<String> {
        self.current_window.clone()
    }

    /// Get window count
    ///
    /// # Returns
    ///
    /// Number of open windows
    pub fn window_count(&self) -> usize {
        self.window_handles.len()
    }
}

/// Create a minimal 1x1 transparent PNG for placeholder screenshots
fn create_placeholder_screenshot() -> Vec<u8> {
    vec![
        0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, // PNG signature
        0x00, 0x00, 0x00, 0x0D, 0x49, 0x48, 0x44, 0x52, // IHDR chunk
        0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, // 1x1 dimensions
        0x08, 0x06, 0x00, 0x00, 0x00, 0x1F, 0x15, 0xC4,
        0x89, 0x00, 0x00, 0x00, 0x0A, 0x49, 0x44, 0x41, // IDAT chunk
        0x54, 0x78, 0x9C, 0x63, 0x00, 0x01, 0x00, 0x00,
        0x05, 0x00, 0x01, 0x0D, 0x0A, 0x2D, 0xB4, 0x00, // Image data
        0x00, 0x00, 0x00, 0x49, 0x45, 0x4E, 0x44, 0xAE, // IEND chunk
        0x42, 0x60, 0x82,
    ]
}

/// Browser capabilities per W3C WebDriver spec
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Capabilities {
    pub browser_name: String,
    pub browser_version: String,
    pub platform_name: String,
    pub accept_insecure_certs: bool,
    pub page_load_strategy: PageLoadStrategy,
    pub timeouts: Timeouts,
}

impl Default for Capabilities {
    fn default() -> Self {
        Self {
            browser_name: "frankenbrowser".to_string(),
            browser_version: "0.1.0".to_string(),
            platform_name: "linux".to_string(),
            accept_insecure_certs: false,
            page_load_strategy: PageLoadStrategy::Normal,
            timeouts: Timeouts::default(),
        }
    }
}

/// Page load strategy
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PageLoadStrategy {
    #[default]
    Normal,
    Eager,
    None,
}

/// Timeouts configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timeouts {
    #[serde(default = "default_script_timeout")]
    pub script: u64,
    #[serde(default = "default_page_load_timeout")]
    pub page_load: u64,
    #[serde(default = "default_implicit_timeout")]
    pub implicit: u64,
}

fn default_script_timeout() -> u64 {
    30000
}

fn default_page_load_timeout() -> u64 {
    300000
}

fn default_implicit_timeout() -> u64 {
    0
}

impl Default for Timeouts {
    fn default() -> Self {
        Self {
            script: default_script_timeout(),
            page_load: default_page_load_timeout(),
            implicit: default_implicit_timeout(),
        }
    }
}

/// Session manager for managing active WebDriver sessions
#[derive(Clone)]
pub struct SessionManager {
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<Session>>>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new session
    pub fn create_session(&self, capabilities: Capabilities) -> Result<String> {
        let session = Session::new(capabilities);
        let session_id = session.id.clone();

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.clone(), Arc::new(Mutex::new(session)));

        Ok(session_id)
    }

    /// Create a new session with browser components
    pub fn create_session_with_browser(
        &self,
        capabilities: Capabilities,
        browser_engine: BrowserEngine,
        webview: WebViewWrapper,
    ) -> Result<String> {
        let session = Session::new_with_browser(capabilities, browser_engine, webview);
        let session_id = session.id.clone();

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.clone(), Arc::new(Mutex::new(session)));

        Ok(session_id)
    }

    /// Get a session by ID (returns Arc<Mutex<Session>> for shared access)
    pub fn get_session(&self, session_id: &str) -> Result<Arc<Mutex<Session>>> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))
    }

    /// Update a session (no longer needed - sessions are modified in place via Arc<Mutex<>>)
    #[deprecated(note = "Sessions are now modified in place via Arc<Mutex<>>")]
    pub fn update_session(&self, _session: Session) -> Result<()> {
        Ok(())
    }

    /// Delete a session
    pub fn delete_session(&self, session_id: &str) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions
            .remove(session_id)
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))?;
        Ok(())
    }

    /// Get all active session IDs
    pub fn list_sessions(&self) -> Vec<String> {
        let sessions = self.sessions.lock().unwrap();
        sessions.keys().cloned().collect()
    }

    /// Count active sessions
    pub fn session_count(&self) -> usize {
        let sessions = self.sessions.lock().unwrap();
        sessions.len()
    }
}

impl Default for SessionManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_creation() {
        let caps = Capabilities::default();
        let session = Session::new(caps);
        assert!(!session.id.is_empty());
        assert!(!session.window_handle.is_empty());
    }

    #[test]
    fn test_session_navigate() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        session.navigate("https://www.example.com".to_string()).unwrap();
        assert_eq!(session.get_url(), Some("https://www.example.com".to_string()));
    }

    #[test]
    fn test_session_navigate_invalid_url() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let result = session.navigate("not a url".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_session_manager_create() {
        let manager = SessionManager::new();
        let caps = Capabilities::default();

        let session_id = manager.create_session(caps).unwrap();
        assert_eq!(manager.session_count(), 1);
        assert!(manager.list_sessions().contains(&session_id));
    }

    #[test]
    fn test_session_manager_get() {
        let manager = SessionManager::new();
        let caps = Capabilities::default();

        let session_id = manager.create_session(caps).unwrap();
        let retrieved = manager.get_session(&session_id).unwrap();
        let session = retrieved.lock().unwrap();
        assert_eq!(session.id, session_id);
    }

    #[test]
    fn test_session_manager_delete() {
        let manager = SessionManager::new();
        let caps = Capabilities::default();

        let session_id = manager.create_session(caps).unwrap();
        assert_eq!(manager.session_count(), 1);

        manager.delete_session(&session_id).unwrap();
        assert_eq!(manager.session_count(), 0);
    }

    #[test]
    fn test_session_manager_get_nonexistent() {
        let manager = SessionManager::new();
        let result = manager.get_session("nonexistent");
        assert!(result.is_err());
    }

    #[test]
    fn test_capabilities_default() {
        let caps = Capabilities::default();
        assert_eq!(caps.browser_name, "frankenbrowser");
        assert_eq!(caps.browser_version, "0.1.0");
        assert_eq!(caps.platform_name, "linux");
    }

    #[test]
    fn test_timeouts_default() {
        let timeouts = Timeouts::default();
        assert_eq!(timeouts.script, 30000);
        assert_eq!(timeouts.page_load, 300000);
        assert_eq!(timeouts.implicit, 0);
    }

    // =================================================================
    // Window Management Tests
    // =================================================================

    #[test]
    fn test_session_has_initial_window_handle() {
        let caps = Capabilities::default();
        let session = Session::new(caps);

        assert_eq!(session.window_count(), 1);
        assert!(session.get_current_window().is_some());
    }

    #[test]
    fn test_new_window_handle() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let initial_count = session.window_count();
        let new_handle = session.new_window_handle(None);

        assert!(!new_handle.is_empty());
        assert_eq!(session.window_count(), initial_count + 1);
        assert!(session.get_window_handle(&new_handle).is_some());
    }

    #[test]
    fn test_new_window_handle_with_tab_id() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let new_handle = session.new_window_handle(Some(42));
        let window = session.get_window_handle(&new_handle).unwrap();

        assert_eq!(window.tab_id, 42);
    }

    #[test]
    fn test_get_all_window_handles() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let handles = session.get_all_window_handles();
        assert_eq!(handles.len(), 1);

        let handle2 = session.new_window_handle(None);
        let handle3 = session.new_window_handle(None);

        let handles = session.get_all_window_handles();
        assert_eq!(handles.len(), 3);
        assert!(handles.contains(&handle2));
        assert!(handles.contains(&handle3));
    }

    #[test]
    fn test_switch_to_window() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let handle1 = session.get_current_window().unwrap();
        let handle2 = session.new_window_handle(None);

        session.switch_to_window(&handle2).unwrap();
        assert_eq!(session.get_current_window(), Some(handle2.clone()));

        session.switch_to_window(&handle1).unwrap();
        assert_eq!(session.get_current_window(), Some(handle1));
    }

    #[test]
    fn test_switch_to_invalid_window() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let result = session.switch_to_window("invalid-handle");
        assert!(result.is_err());

        match result {
            Err(Error::NoSuchWindow(_)) => (),
            _ => panic!("Expected NoSuchWindow error"),
        }
    }

    #[test]
    fn test_close_window() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let handle1 = session.get_current_window().unwrap();
        let handle2 = session.new_window_handle(None);

        assert_eq!(session.window_count(), 2);

        let remaining = session.close_window(&handle2).unwrap();
        assert_eq!(session.window_count(), 1);
        assert_eq!(remaining.len(), 1);
        assert!(remaining.contains(&handle1));
    }

    #[test]
    fn test_close_window_switches_to_another() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let handle1 = session.get_current_window().unwrap();
        let handle2 = session.new_window_handle(None);

        // Close current window (handle1)
        session.close_window(&handle1).unwrap();

        // Should switch to handle2
        assert_eq!(session.get_current_window(), Some(handle2));
    }

    #[test]
    fn test_close_invalid_window() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let result = session.close_window("invalid-handle");
        assert!(result.is_err());

        match result {
            Err(Error::NoSuchWindow(_)) => (),
            _ => panic!("Expected NoSuchWindow error"),
        }
    }

    #[test]
    fn test_close_last_window() {
        let caps = Capabilities::default();
        let mut session = Session::new(caps);

        let handle = session.get_current_window().unwrap();
        let remaining = session.close_window(&handle).unwrap();

        assert_eq!(session.window_count(), 0);
        assert_eq!(remaining.len(), 0);
        assert!(session.get_current_window().is_none());
    }

    #[test]
    fn test_window_handle_struct() {
        let handle = WindowHandle::new("test-uuid".to_string(), 42);

        assert_eq!(handle.handle, "test-uuid");
        assert_eq!(handle.tab_id, 42);
        assert!(handle.title.is_none());
        assert!(handle.url.is_none());
    }
}
