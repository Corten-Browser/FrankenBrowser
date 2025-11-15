//! WebDriver session management

use crate::errors::{Error, Result};
use browser_core::BrowserEngine;
use webview_integration::WebViewWrapper;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

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
}

impl Session {
    /// Create a new session with random ID
    pub fn new(capabilities: Capabilities) -> Self {
        let id = Uuid::new_v4().to_string();
        let window_handle = Uuid::new_v4().to_string();

        Self {
            id,
            capabilities,
            current_url: None,
            window_handle,
            browser_engine: None,
            webview: None,
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

        Self {
            id,
            capabilities,
            current_url: None,
            window_handle,
            browser_engine: Some(Arc::new(Mutex::new(browser_engine))),
            webview: Some(Arc::new(Mutex::new(webview))),
        }
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: String) -> Result<()> {
        // Validate URL
        url::Url::parse(&url)
            .map_err(|e| Error::InvalidArgument(format!("Invalid URL: {}", e)))?;

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
}
