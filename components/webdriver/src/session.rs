//! WebDriver session management

use crate::errors::{Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

/// WebDriver session representing a browser instance
#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub capabilities: Capabilities,
    pub current_url: Option<String>,
    pub window_handle: String,
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
        }
    }

    /// Navigate to a URL
    pub fn navigate(&mut self, url: String) -> Result<()> {
        // Validate URL
        url::Url::parse(&url)
            .map_err(|e| Error::InvalidArgument(format!("Invalid URL: {}", e)))?;

        self.current_url = Some(url);
        Ok(())
    }

    /// Get current URL
    pub fn get_url(&self) -> Option<&str> {
        self.current_url.as_deref()
    }
}

/// Browser capabilities per W3C WebDriver spec
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Capabilities {
    pub browser_name: String,
    pub browser_version: String,
    pub platform_name: String,
    #[serde(default)]
    pub accept_insecure_certs: bool,
    #[serde(default)]
    pub page_load_strategy: PageLoadStrategy,
    #[serde(default)]
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
    sessions: Arc<Mutex<HashMap<String, Session>>>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new() -> Self {
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Create a new session
    pub fn create_session(&self, capabilities: Capabilities) -> Result<Session> {
        let session = Session::new(capabilities);
        let session_id = session.id.clone();

        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session_id.clone(), session.clone());

        Ok(session)
    }

    /// Get a session by ID
    pub fn get_session(&self, session_id: &str) -> Result<Session> {
        let sessions = self.sessions.lock().unwrap();
        sessions
            .get(session_id)
            .cloned()
            .ok_or_else(|| Error::SessionNotFound(session_id.to_string()))
    }

    /// Update a session
    pub fn update_session(&self, session: Session) -> Result<()> {
        let mut sessions = self.sessions.lock().unwrap();
        sessions.insert(session.id.clone(), session);
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
        assert_eq!(session.get_url(), Some("https://www.example.com"));
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

        let session = manager.create_session(caps).unwrap();
        assert_eq!(manager.session_count(), 1);
        assert!(manager.list_sessions().contains(&session.id));
    }

    #[test]
    fn test_session_manager_get() {
        let manager = SessionManager::new();
        let caps = Capabilities::default();

        let session = manager.create_session(caps).unwrap();
        let retrieved = manager.get_session(&session.id).unwrap();
        assert_eq!(retrieved.id, session.id);
    }

    #[test]
    fn test_session_manager_delete() {
        let manager = SessionManager::new();
        let caps = Capabilities::default();

        let session = manager.create_session(caps).unwrap();
        assert_eq!(manager.session_count(), 1);

        manager.delete_session(&session.id).unwrap();
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
