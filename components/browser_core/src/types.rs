//! Type definitions for browser_core component

use crate::errors::{Error, Result};
use crate::navigation::Navigator;
use config_manager::Config;
use message_bus::MessageSender;
use network_stack::NetworkStack;
use rusqlite::Connection;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use url::Url;

/// A bookmark entry
#[derive(Debug, Clone, PartialEq)]
pub struct Bookmark {
    /// Unique identifier
    pub id: i64,
    /// Bookmark URL
    pub url: String,
    /// Bookmark title
    pub title: String,
    /// Creation timestamp (Unix timestamp)
    pub created_at: i64,
}

/// A history entry
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryEntry {
    /// Unique identifier
    pub id: i64,
    /// URL
    pub url: String,
    /// Page title
    pub title: String,
    /// Number of visits
    pub visit_count: i32,
    /// Last visit timestamp (Unix timestamp)
    pub last_visit: i64,
}

/// Tab navigation state
#[derive(Debug, Clone)]
struct TabState {
    /// History stack (URLs visited)
    history: Vec<Url>,
    /// Current position in history
    position: usize,
    /// Current page title
    #[allow(dead_code)]
    title: Option<String>,
}

impl TabState {
    fn new() -> Self {
        Self {
            history: Vec::new(),
            position: 0,
            title: None,
        }
    }

    fn current_url(&self) -> Option<&Url> {
        if self.history.is_empty() {
            None
        } else {
            self.history.get(self.position)
        }
    }

    fn can_go_back(&self) -> bool {
        self.position > 0
    }

    fn can_go_forward(&self) -> bool {
        self.position < self.history.len().saturating_sub(1)
    }

    fn navigate(&mut self, url: Url) {
        // Remove forward history when navigating to a new URL
        self.history.truncate(self.position + 1);
        self.history.push(url);
        self.position = self.history.len() - 1;
    }

    fn go_back(&mut self) -> Option<&Url> {
        if self.can_go_back() {
            self.position -= 1;
            self.current_url()
        } else {
            None
        }
    }

    fn go_forward(&mut self) -> Option<&Url> {
        if self.can_go_forward() {
            self.position += 1;
            self.current_url()
        } else {
            None
        }
    }
}

/// Browser engine
///
/// Manages navigation, history, and bookmarks.
pub struct BrowserEngine {
    /// Configuration
    #[allow(dead_code)]
    config: Config,
    /// Network stack for fetching content
    #[allow(dead_code)]
    network: NetworkStack,
    /// Message bus for sending messages
    #[allow(dead_code)]
    message_bus: Box<dyn MessageSender>,
    /// Navigator for protocol handling
    navigator: Arc<Mutex<Navigator>>,
    /// Per-tab navigation state
    tabs: Arc<Mutex<HashMap<u32, TabState>>>,
    /// History database connection
    history_db: Arc<Mutex<Connection>>,
    /// Bookmarks database connection
    bookmarks_db: Arc<Mutex<Connection>>,
}

impl BrowserEngine {
    /// Create a new browser engine
    ///
    /// # Arguments
    ///
    /// * `config` - Configuration
    /// * `network` - Network stack for fetching content
    /// * `message_bus` - Message bus for sending messages
    ///
    /// # Returns
    ///
    /// Returns a `Result<BrowserEngine>`.
    ///
    /// # Errors
    ///
    /// Returns an error if database initialization fails.
    pub fn new(
        config: Config,
        network: NetworkStack,
        message_bus: Box<dyn MessageSender>,
    ) -> Result<Self> {
        // Create in-memory databases for testing
        let history_db = Connection::open_in_memory()?;
        let bookmarks_db = Connection::open_in_memory()?;

        // Initialize schema
        Self::init_history_schema(&history_db)?;
        Self::init_bookmarks_schema(&bookmarks_db)?;

        Ok(Self {
            config,
            network,
            message_bus,
            navigator: Arc::new(Mutex::new(Navigator::new())),
            tabs: Arc::new(Mutex::new(HashMap::new())),
            history_db: Arc::new(Mutex::new(history_db)),
            bookmarks_db: Arc::new(Mutex::new(bookmarks_db)),
        })
    }

    /// Initialize history database schema
    fn init_history_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                visit_count INTEGER NOT NULL DEFAULT 1,
                last_visit INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute("CREATE INDEX idx_history_url ON history(url)", [])?;

        Ok(())
    }

    /// Initialize bookmarks database schema
    fn init_bookmarks_schema(conn: &Connection) -> Result<()> {
        conn.execute(
            "CREATE TABLE bookmarks (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                url TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        conn.execute("CREATE INDEX idx_bookmarks_url ON bookmarks(url)", [])?;

        Ok(())
    }

    /// Navigate to a URL
    ///
    /// # Arguments
    ///
    /// * `tab_id` - Tab identifier
    /// * `url` - URL to navigate to
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - URL validation fails
    /// - Protocol is unsupported
    /// - Network fetch fails
    pub fn navigate(&mut self, tab_id: u32, url: Url) -> Result<()> {
        // Use Navigator to handle protocol-specific navigation
        let nav_result = {
            let mut navigator = self.navigator.lock().unwrap();
            navigator.navigate(url.clone())
        };

        // Process navigation result
        match nav_result {
            Ok(_state) => {
                // Update tab navigation state
                {
                    let mut tabs = self.tabs.lock().unwrap();
                    let tab_state = tabs.entry(tab_id).or_insert_with(TabState::new);
                    tab_state.navigate(url.clone());
                }

                // Add to history
                self.add_to_history(url.as_str(), "")?;

                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    /// Go back in history
    ///
    /// # Arguments
    ///
    /// * `tab_id` - Tab identifier
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tab not found
    /// - No history available
    pub fn go_back(&mut self, tab_id: u32) -> Result<()> {
        let mut tabs = self.tabs.lock().unwrap();
        let tab_state = tabs.get_mut(&tab_id).ok_or(Error::TabNotFound(tab_id))?;

        if !tab_state.can_go_back() {
            return Err(Error::NoHistory(tab_id));
        }

        tab_state.go_back();

        Ok(())
    }

    /// Go forward in history
    ///
    /// # Arguments
    ///
    /// * `tab_id` - Tab identifier
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tab not found
    /// - No forward history available
    pub fn go_forward(&mut self, tab_id: u32) -> Result<()> {
        let mut tabs = self.tabs.lock().unwrap();
        let tab_state = tabs.get_mut(&tab_id).ok_or(Error::TabNotFound(tab_id))?;

        if !tab_state.can_go_forward() {
            return Err(Error::NoForwardHistory(tab_id));
        }

        tab_state.go_forward();

        Ok(())
    }

    /// Reload current page
    ///
    /// # Arguments
    ///
    /// * `tab_id` - Tab identifier
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Tab not found
    /// - No current page
    pub fn reload(&mut self, tab_id: u32) -> Result<()> {
        let tabs = self.tabs.lock().unwrap();
        let tab_state = tabs.get(&tab_id).ok_or(Error::TabNotFound(tab_id))?;

        let _current_url = tab_state
            .current_url()
            .ok_or(Error::NoCurrentPage(tab_id))?;

        // In a real implementation, we would fetch the URL again
        // For now, just verify we have a current page

        Ok(())
    }

    /// Add a bookmark
    ///
    /// # Arguments
    ///
    /// * `url` - URL to bookmark
    /// * `title` - Bookmark title
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` on success.
    ///
    /// # Errors
    ///
    /// Returns an error if database operation fails.
    pub fn add_bookmark(&mut self, url: Url, title: String) -> Result<()> {
        let db = self.bookmarks_db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        db.execute(
            "INSERT OR REPLACE INTO bookmarks (url, title, created_at) VALUES (?1, ?2, ?3)",
            [url.as_str(), &title, &now.to_string()],
        )?;

        Ok(())
    }

    /// Get all bookmarks
    ///
    /// # Returns
    ///
    /// Returns a vector of bookmarks.
    pub fn get_bookmarks(&self) -> Vec<Bookmark> {
        let db = self.bookmarks_db.lock().unwrap();

        let mut stmt = match db
            .prepare("SELECT id, url, title, created_at FROM bookmarks ORDER BY created_at DESC")
        {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let bookmarks = stmt
            .query_map([], |row| {
                Ok(Bookmark {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    created_at: row.get(3)?,
                })
            })
            .unwrap();

        bookmarks.filter_map(|b| b.ok()).collect()
    }

    /// Get browsing history
    ///
    /// # Returns
    ///
    /// Returns a vector of history entries.
    pub fn get_history(&self) -> Vec<HistoryEntry> {
        let db = self.history_db.lock().unwrap();

        let mut stmt = match db.prepare(
            "SELECT id, url, title, visit_count, last_visit FROM history ORDER BY last_visit DESC",
        ) {
            Ok(stmt) => stmt,
            Err(_) => return Vec::new(),
        };

        let entries = stmt
            .query_map([], |row| {
                Ok(HistoryEntry {
                    id: row.get(0)?,
                    url: row.get(1)?,
                    title: row.get(2)?,
                    visit_count: row.get(3)?,
                    last_visit: row.get(4)?,
                })
            })
            .unwrap();

        entries.filter_map(|e| e.ok()).collect()
    }

    /// Add URL to history
    fn add_to_history(&mut self, url: &str, title: &str) -> Result<()> {
        let db = self.history_db.lock().unwrap();
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Try to update existing entry
        let updated = db.execute(
            "UPDATE history SET visit_count = visit_count + 1, last_visit = ?1 WHERE url = ?2",
            [now.to_string(), url.to_string()],
        )?;

        // If no rows updated, insert new entry
        if updated == 0 {
            db.execute(
                "INSERT INTO history (url, title, visit_count, last_visit) VALUES (?1, ?2, 1, ?3)",
                [url, title, &now.to_string()],
            )?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use config_manager::Config;
    use message_bus::MessageBus;
    use network_stack::NetworkStack;

    // Helper to create test engine
    fn create_test_engine() -> BrowserEngine {
        let config = Config::default();
        let mut bus = MessageBus::new();
        bus.start().unwrap();
        let sender = bus.sender();
        let network = NetworkStack::new(config.network_config(), sender).unwrap();

        let mut bus2 = MessageBus::new();
        bus2.start().unwrap();

        BrowserEngine::new(config, network, bus2.sender()).unwrap()
    }

    // ========================================
    // RED PHASE: Tests for Bookmark type
    // ========================================

    #[test]
    fn test_bookmark_creation() {
        let bookmark = Bookmark {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            created_at: 1234567890,
        };

        assert_eq!(bookmark.id, 1);
        assert_eq!(bookmark.url, "https://example.com");
        assert_eq!(bookmark.title, "Example");
        assert_eq!(bookmark.created_at, 1234567890);
    }

    #[test]
    fn test_bookmark_clone() {
        let bookmark = Bookmark {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            created_at: 1234567890,
        };

        let cloned = bookmark.clone();
        assert_eq!(bookmark, cloned);
    }

    #[test]
    fn test_bookmark_debug() {
        let bookmark = Bookmark {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            created_at: 1234567890,
        };

        let debug_str = format!("{:?}", bookmark);
        assert!(debug_str.contains("Bookmark"));
        assert!(debug_str.contains("example.com"));
    }

    // ========================================
    // RED PHASE: Tests for HistoryEntry type
    // ========================================

    #[test]
    fn test_history_entry_creation() {
        let entry = HistoryEntry {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            visit_count: 5,
            last_visit: 1234567890,
        };

        assert_eq!(entry.id, 1);
        assert_eq!(entry.url, "https://example.com");
        assert_eq!(entry.title, "Example");
        assert_eq!(entry.visit_count, 5);
        assert_eq!(entry.last_visit, 1234567890);
    }

    #[test]
    fn test_history_entry_clone() {
        let entry = HistoryEntry {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            visit_count: 5,
            last_visit: 1234567890,
        };

        let cloned = entry.clone();
        assert_eq!(entry, cloned);
    }

    #[test]
    fn test_history_entry_debug() {
        let entry = HistoryEntry {
            id: 1,
            url: "https://example.com".to_string(),
            title: "Example".to_string(),
            visit_count: 5,
            last_visit: 1234567890,
        };

        let debug_str = format!("{:?}", entry);
        assert!(debug_str.contains("HistoryEntry"));
        assert!(debug_str.contains("example.com"));
    }

    // ========================================
    // RED PHASE: Tests for BrowserEngine::new
    // ========================================

    #[test]
    fn test_browser_engine_new() {
        let engine = create_test_engine();
        // Should create successfully
        drop(engine);
    }

    #[test]
    fn test_browser_engine_initializes_databases() {
        let engine = create_test_engine();

        // Verify history database initialized
        let history = engine.get_history();
        assert_eq!(history.len(), 0);

        // Verify bookmarks database initialized
        let bookmarks = engine.get_bookmarks();
        assert_eq!(bookmarks.len(), 0);
    }

    // ========================================
    // RED PHASE: Tests for Navigation
    // ========================================

    #[test]
    fn test_navigate_creates_tab() {
        let mut engine = create_test_engine();
        let url = Url::parse("https://example.com").unwrap();

        let result = engine.navigate(1, url);
        assert!(result.is_ok());
    }

    #[test]
    fn test_navigate_adds_to_history() {
        let mut engine = create_test_engine();
        let url = Url::parse("https://example.com").unwrap();

        engine.navigate(1, url).unwrap();

        let history = engine.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].url, "https://example.com/");
    }

    #[test]
    fn test_navigate_multiple_urls() {
        let mut engine = create_test_engine();

        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();
        engine
            .navigate(1, Url::parse("https://example.org").unwrap())
            .unwrap();

        let history = engine.get_history();
        assert_eq!(history.len(), 2);
    }

    #[test]
    fn test_go_back_without_navigation_fails() {
        let mut engine = create_test_engine();

        let result = engine.go_back(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::TabNotFound(_)));
    }

    #[test]
    fn test_go_back_without_history_fails() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();

        let result = engine.go_back(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoHistory(_)));
    }

    #[test]
    fn test_go_back_success() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();
        engine
            .navigate(1, Url::parse("https://example.org").unwrap())
            .unwrap();

        let result = engine.go_back(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_go_forward_without_navigation_fails() {
        let mut engine = create_test_engine();

        let result = engine.go_forward(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::TabNotFound(_)));
    }

    #[test]
    fn test_go_forward_without_forward_history_fails() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();

        let result = engine.go_forward(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::NoForwardHistory(_)));
    }

    #[test]
    fn test_go_forward_success() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();
        engine
            .navigate(1, Url::parse("https://example.org").unwrap())
            .unwrap();
        engine.go_back(1).unwrap();

        let result = engine.go_forward(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_reload_without_navigation_fails() {
        let mut engine = create_test_engine();

        let result = engine.reload(1);
        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), Error::TabNotFound(_)));
    }

    #[test]
    fn test_reload_success() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();

        let result = engine.reload(1);
        assert!(result.is_ok());
    }

    #[test]
    fn test_navigate_clears_forward_history() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();
        engine
            .navigate(1, Url::parse("https://example.org").unwrap())
            .unwrap();
        engine.go_back(1).unwrap();

        // Now navigate to a new URL - should clear forward history
        engine
            .navigate(1, Url::parse("https://example.net").unwrap())
            .unwrap();

        // Go forward should fail
        let result = engine.go_forward(1);
        assert!(result.is_err());
    }

    // ========================================
    // RED PHASE: Tests for Bookmarks
    // ========================================

    #[test]
    fn test_add_bookmark() {
        let mut engine = create_test_engine();
        let url = Url::parse("https://example.com").unwrap();

        let result = engine.add_bookmark(url, "Example".to_string());
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_bookmarks_empty() {
        let engine = create_test_engine();
        let bookmarks = engine.get_bookmarks();
        assert_eq!(bookmarks.len(), 0);
    }

    #[test]
    fn test_get_bookmarks_after_adding() {
        let mut engine = create_test_engine();
        engine
            .add_bookmark(
                Url::parse("https://example.com").unwrap(),
                "Example".to_string(),
            )
            .unwrap();

        let bookmarks = engine.get_bookmarks();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].url, "https://example.com/");
        assert_eq!(bookmarks[0].title, "Example");
    }

    #[test]
    fn test_add_multiple_bookmarks() {
        let mut engine = create_test_engine();
        engine
            .add_bookmark(
                Url::parse("https://example.com").unwrap(),
                "Example 1".to_string(),
            )
            .unwrap();
        engine
            .add_bookmark(
                Url::parse("https://example.org").unwrap(),
                "Example 2".to_string(),
            )
            .unwrap();

        let bookmarks = engine.get_bookmarks();
        assert_eq!(bookmarks.len(), 2);
    }

    #[test]
    fn test_add_duplicate_bookmark_replaces() {
        let mut engine = create_test_engine();
        let url = Url::parse("https://example.com").unwrap();

        engine
            .add_bookmark(url.clone(), "Example 1".to_string())
            .unwrap();
        engine.add_bookmark(url, "Example 2".to_string()).unwrap();

        let bookmarks = engine.get_bookmarks();
        assert_eq!(bookmarks.len(), 1);
        assert_eq!(bookmarks[0].title, "Example 2");
    }

    // ========================================
    // RED PHASE: Tests for History
    // ========================================

    #[test]
    fn test_get_history_empty() {
        let engine = create_test_engine();
        let history = engine.get_history();
        assert_eq!(history.len(), 0);
    }

    #[test]
    fn test_get_history_after_navigation() {
        let mut engine = create_test_engine();
        engine
            .navigate(1, Url::parse("https://example.com").unwrap())
            .unwrap();

        let history = engine.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].url, "https://example.com/");
        assert_eq!(history[0].visit_count, 1);
    }

    #[test]
    fn test_history_visit_count_increments() {
        let mut engine = create_test_engine();
        let url = Url::parse("https://example.com").unwrap();

        engine.navigate(1, url.clone()).unwrap();
        engine.navigate(2, url).unwrap();

        let history = engine.get_history();
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].visit_count, 2);
    }

    #[test]
    fn test_history_last_visit_updates() {
        let mut engine = create_test_engine();
        let url = Url::parse("https://example.com").unwrap();

        engine.navigate(1, url.clone()).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        engine.navigate(2, url).unwrap();

        let history = engine.get_history();
        assert_eq!(history.len(), 1);
        // Last visit should be recent
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        assert!(history[0].last_visit <= now);
        assert!(history[0].last_visit > now - 10);
    }

    // ========================================
    // RED PHASE: Tests for TabState
    // ========================================

    #[test]
    fn test_tab_state_new() {
        let state = TabState::new();
        assert_eq!(state.history.len(), 0);
        assert_eq!(state.position, 0);
        assert!(state.current_url().is_none());
    }

    #[test]
    fn test_tab_state_navigate() {
        let mut state = TabState::new();
        let url = Url::parse("https://example.com").unwrap();

        state.navigate(url.clone());

        assert_eq!(state.history.len(), 1);
        assert_eq!(state.position, 0);
        assert_eq!(state.current_url(), Some(&url));
    }

    #[test]
    fn test_tab_state_can_go_back() {
        let mut state = TabState::new();
        assert!(!state.can_go_back());

        state.navigate(Url::parse("https://example.com").unwrap());
        assert!(!state.can_go_back());

        state.navigate(Url::parse("https://example.org").unwrap());
        assert!(state.can_go_back());
    }

    #[test]
    fn test_tab_state_can_go_forward() {
        let mut state = TabState::new();
        assert!(!state.can_go_forward());

        state.navigate(Url::parse("https://example.com").unwrap());
        assert!(!state.can_go_forward());

        state.navigate(Url::parse("https://example.org").unwrap());
        state.go_back();
        assert!(state.can_go_forward());
    }

    #[test]
    fn test_tab_state_go_back() {
        let mut state = TabState::new();
        let url1 = Url::parse("https://example.com").unwrap();
        let url2 = Url::parse("https://example.org").unwrap();

        state.navigate(url1.clone());
        state.navigate(url2);

        let back_url = state.go_back();
        assert_eq!(back_url, Some(&url1));
        assert_eq!(state.position, 0);
    }

    #[test]
    fn test_tab_state_go_forward() {
        let mut state = TabState::new();
        let url1 = Url::parse("https://example.com").unwrap();
        let url2 = Url::parse("https://example.org").unwrap();

        state.navigate(url1);
        state.navigate(url2.clone());
        state.go_back();

        let forward_url = state.go_forward();
        assert_eq!(forward_url, Some(&url2));
        assert_eq!(state.position, 1);
    }

    #[test]
    fn test_tab_state_navigate_clears_forward_history() {
        let mut state = TabState::new();
        state.navigate(Url::parse("https://example.com").unwrap());
        state.navigate(Url::parse("https://example.org").unwrap());
        state.navigate(Url::parse("https://example.net").unwrap());
        state.go_back();
        state.go_back();

        // Now at position 0, history has 3 entries
        assert_eq!(state.position, 0);
        assert_eq!(state.history.len(), 3);

        // Navigate to new URL - should truncate forward history
        state.navigate(Url::parse("https://example.edu").unwrap());

        assert_eq!(state.history.len(), 2);
        assert_eq!(state.position, 1);
        assert!(!state.can_go_forward());
    }
}
