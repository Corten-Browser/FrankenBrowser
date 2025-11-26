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

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestStatus {
    /// Test passed
    Passed,
    /// Test failed
    Failed,
    /// Test was skipped
    Skipped,
    /// Test timed out
    Timeout,
    /// Test errored (infrastructure failure)
    Error,
}

impl TestStatus {
    /// Convert to string representation
    pub fn as_str(&self) -> &'static str {
        match self {
            TestStatus::Passed => "passed",
            TestStatus::Failed => "failed",
            TestStatus::Skipped => "skipped",
            TestStatus::Timeout => "timeout",
            TestStatus::Error => "error",
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "passed" | "pass" => Some(TestStatus::Passed),
            "failed" | "fail" => Some(TestStatus::Failed),
            "skipped" | "skip" => Some(TestStatus::Skipped),
            "timeout" => Some(TestStatus::Timeout),
            "error" => Some(TestStatus::Error),
            _ => None,
        }
    }
}

/// A test result entry
#[derive(Debug, Clone, PartialEq)]
pub struct TestResult {
    /// Unique identifier
    pub id: i64,
    /// Test suite name (e.g., "WPT", "ACID1", "unit")
    pub suite: String,
    /// Test name/path
    pub name: String,
    /// Test status
    pub status: TestStatus,
    /// Execution time in milliseconds
    pub duration_ms: i64,
    /// Error message if failed
    pub error_message: Option<String>,
    /// Run timestamp (Unix timestamp)
    pub run_at: i64,
    /// Browser version
    pub browser_version: String,
}

/// A performance metric entry
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceMetric {
    /// Unique identifier
    pub id: i64,
    /// Metric name (e.g., "page_load_time", "memory_usage")
    pub name: String,
    /// Metric value
    pub value: f64,
    /// Unit of measurement (e.g., "ms", "MB", "count")
    pub unit: String,
    /// URL or context this metric was collected from
    pub context: Option<String>,
    /// Collection timestamp (Unix timestamp)
    pub collected_at: i64,
}

/// Test result database for storing test runs and performance metrics
pub struct TestResultDatabase {
    /// Database connection
    conn: Connection,
}

impl TestResultDatabase {
    /// Create a new test result database
    ///
    /// # Arguments
    ///
    /// * `path` - Optional path for persistent storage. If None, uses in-memory database.
    ///
    /// # Returns
    ///
    /// Returns a new TestResultDatabase instance.
    ///
    /// # Errors
    ///
    /// Returns an error if database creation or schema initialization fails.
    pub fn new(path: Option<&str>) -> Result<Self> {
        let conn = match path {
            Some(p) => Connection::open(p)?,
            None => Connection::open_in_memory()?,
        };

        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    /// Initialize database schema
    fn init_schema(&self) -> Result<()> {
        // Test results table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS test_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                suite TEXT NOT NULL,
                name TEXT NOT NULL,
                status TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                error_message TEXT,
                run_at INTEGER NOT NULL,
                browser_version TEXT NOT NULL
            )",
            [],
        )?;

        // Performance metrics table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS performance_metrics (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                value REAL NOT NULL,
                unit TEXT NOT NULL,
                context TEXT,
                collected_at INTEGER NOT NULL
            )",
            [],
        )?;

        // Indexes for common queries
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_test_results_suite ON test_results(suite)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_test_results_run_at ON test_results(run_at)",
            [],
        )?;
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_performance_metrics_name ON performance_metrics(name)",
            [],
        )?;

        Ok(())
    }

    /// Record a test result
    ///
    /// # Arguments
    ///
    /// * `suite` - Test suite name
    /// * `name` - Test name
    /// * `status` - Test status
    /// * `duration_ms` - Execution time in milliseconds
    /// * `error_message` - Optional error message
    /// * `browser_version` - Browser version string
    pub fn record_test_result(
        &self,
        suite: &str,
        name: &str,
        status: TestStatus,
        duration_ms: i64,
        error_message: Option<&str>,
        browser_version: &str,
    ) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO test_results (suite, name, status, duration_ms, error_message, run_at, browser_version)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            rusqlite::params![
                suite,
                name,
                status.as_str(),
                duration_ms,
                error_message,
                now,
                browser_version
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Record a performance metric
    ///
    /// # Arguments
    ///
    /// * `name` - Metric name
    /// * `value` - Metric value
    /// * `unit` - Unit of measurement
    /// * `context` - Optional context (e.g., URL)
    pub fn record_metric(
        &self,
        name: &str,
        value: f64,
        unit: &str,
        context: Option<&str>,
    ) -> Result<i64> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        self.conn.execute(
            "INSERT INTO performance_metrics (name, value, unit, context, collected_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![name, value, unit, context, now],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get test results for a suite
    pub fn get_test_results(&self, suite: Option<&str>, limit: Option<i64>) -> Vec<TestResult> {
        let limit_val = limit.unwrap_or(100);

        let (query, params): (&str, Vec<Box<dyn rusqlite::ToSql>>) = match suite {
            Some(s) => (
                "SELECT id, suite, name, status, duration_ms, error_message, run_at, browser_version
                 FROM test_results WHERE suite = ?1 ORDER BY run_at DESC LIMIT ?2",
                vec![Box::new(s.to_string()), Box::new(limit_val)],
            ),
            None => (
                "SELECT id, suite, name, status, duration_ms, error_message, run_at, browser_version
                 FROM test_results ORDER BY run_at DESC LIMIT ?1",
                vec![Box::new(limit_val)],
            ),
        };

        let mut stmt = match self.conn.prepare(query) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let results = stmt.query_map(params_refs.as_slice(), |row| {
            let status_str: String = row.get(3)?;
            Ok(TestResult {
                id: row.get(0)?,
                suite: row.get(1)?,
                name: row.get(2)?,
                status: TestStatus::from_str(&status_str).unwrap_or(TestStatus::Error),
                duration_ms: row.get(4)?,
                error_message: row.get(5)?,
                run_at: row.get(6)?,
                browser_version: row.get(7)?,
            })
        });

        match results {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get performance metrics
    pub fn get_metrics(&self, name: Option<&str>, limit: Option<i64>) -> Vec<PerformanceMetric> {
        let limit_val = limit.unwrap_or(100);

        let (query, params): (&str, Vec<Box<dyn rusqlite::ToSql>>) = match name {
            Some(n) => (
                "SELECT id, name, value, unit, context, collected_at
                 FROM performance_metrics WHERE name = ?1 ORDER BY collected_at DESC LIMIT ?2",
                vec![Box::new(n.to_string()), Box::new(limit_val)],
            ),
            None => (
                "SELECT id, name, value, unit, context, collected_at
                 FROM performance_metrics ORDER BY collected_at DESC LIMIT ?1",
                vec![Box::new(limit_val)],
            ),
        };

        let mut stmt = match self.conn.prepare(query) {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };

        let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();

        let results = stmt.query_map(params_refs.as_slice(), |row| {
            Ok(PerformanceMetric {
                id: row.get(0)?,
                name: row.get(1)?,
                value: row.get(2)?,
                unit: row.get(3)?,
                context: row.get(4)?,
                collected_at: row.get(5)?,
            })
        });

        match results {
            Ok(iter) => iter.filter_map(|r| r.ok()).collect(),
            Err(_) => Vec::new(),
        }
    }

    /// Get test summary statistics for a suite
    pub fn get_test_summary(&self, suite: &str) -> TestSummary {
        let mut stmt = match self.conn.prepare(
            "SELECT status, COUNT(*) FROM test_results WHERE suite = ?1 GROUP BY status",
        ) {
            Ok(s) => s,
            Err(_) => {
                return TestSummary::default();
            }
        };

        let mut summary = TestSummary::default();

        if let Ok(rows) = stmt.query_map([suite], |row| {
            let status_str: String = row.get(0)?;
            let count: i64 = row.get(1)?;
            Ok((status_str, count))
        }) {
            for row in rows.flatten() {
                match row.0.as_str() {
                    "passed" | "pass" => summary.passed = row.1 as u64,
                    "failed" | "fail" => summary.failed = row.1 as u64,
                    "skipped" | "skip" => summary.skipped = row.1 as u64,
                    "timeout" => summary.timeout = row.1 as u64,
                    "error" => summary.error = row.1 as u64,
                    _ => {}
                }
            }
        }

        summary.total = summary.passed + summary.failed + summary.skipped + summary.timeout + summary.error;
        summary
    }
}

/// Summary of test results
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TestSummary {
    /// Total number of tests
    pub total: u64,
    /// Number of passed tests
    pub passed: u64,
    /// Number of failed tests
    pub failed: u64,
    /// Number of skipped tests
    pub skipped: u64,
    /// Number of timed out tests
    pub timeout: u64,
    /// Number of errored tests
    pub error: u64,
}

impl TestSummary {
    /// Calculate pass rate as a percentage
    pub fn pass_rate(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.passed as f64 / self.total as f64) * 100.0
    }
}

/// Browser metrics for monitoring performance and resource usage
#[derive(Debug, Clone, Default)]
pub struct BrowserMetrics {
    /// Number of active tabs
    pub active_tabs: u32,
    /// Total network requests made
    pub network_requests: u64,
    /// Network requests blocked by ad blocker
    pub blocked_requests: u64,
    /// Memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Current page load time in milliseconds (0 if no page loaded)
    pub page_load_time_ms: u64,
    /// Number of pages visited this session
    pub pages_visited: u64,
    /// Session start time (Unix timestamp)
    pub session_start: i64,
    /// Last update time (Unix timestamp)
    pub last_updated: i64,
}

impl BrowserMetrics {
    /// Create new metrics instance with current timestamp
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        Self {
            session_start: now,
            last_updated: now,
            ..Default::default()
        }
    }

    /// Increment the network request counter
    pub fn record_request(&mut self) {
        self.network_requests += 1;
        self.update_timestamp();
    }

    /// Increment the blocked request counter
    pub fn record_blocked(&mut self) {
        self.blocked_requests += 1;
        self.update_timestamp();
    }

    /// Record a page navigation
    pub fn record_navigation(&mut self, load_time_ms: u64) {
        self.pages_visited += 1;
        self.page_load_time_ms = load_time_ms;
        self.update_timestamp();
    }

    /// Update the active tab count
    pub fn set_active_tabs(&mut self, count: u32) {
        self.active_tabs = count;
        self.update_timestamp();
    }

    /// Update memory usage
    pub fn set_memory_usage(&mut self, bytes: u64) {
        self.memory_usage_bytes = bytes;
        self.update_timestamp();
    }

    /// Calculate block rate as a percentage
    pub fn block_rate(&self) -> f64 {
        if self.network_requests == 0 {
            return 0.0;
        }
        (self.blocked_requests as f64 / self.network_requests as f64) * 100.0
    }

    /// Get session duration in seconds
    pub fn session_duration_secs(&self) -> i64 {
        self.last_updated - self.session_start
    }

    /// Get memory usage in megabytes
    pub fn memory_usage_mb(&self) -> f64 {
        self.memory_usage_bytes as f64 / (1024.0 * 1024.0)
    }

    /// Update the last_updated timestamp
    fn update_timestamp(&mut self) {
        self.last_updated = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
    }

    /// Create a snapshot for serialization
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            active_tabs: self.active_tabs,
            network_requests: self.network_requests,
            blocked_requests: self.blocked_requests,
            block_rate_percent: self.block_rate(),
            memory_usage_mb: self.memory_usage_mb(),
            page_load_time_ms: self.page_load_time_ms,
            pages_visited: self.pages_visited,
            session_duration_secs: self.session_duration_secs(),
        }
    }
}

/// Serializable metrics snapshot
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetricsSnapshot {
    /// Number of active tabs
    pub active_tabs: u32,
    /// Total network requests made
    pub network_requests: u64,
    /// Network requests blocked by ad blocker
    pub blocked_requests: u64,
    /// Block rate as percentage
    pub block_rate_percent: f64,
    /// Memory usage in megabytes
    pub memory_usage_mb: f64,
    /// Current page load time in milliseconds
    pub page_load_time_ms: u64,
    /// Number of pages visited this session
    pub pages_visited: u64,
    /// Session duration in seconds
    pub session_duration_secs: i64,
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

    // ========================================
    // Tests for TestResultDatabase
    // ========================================

    #[test]
    fn test_test_result_database_new() {
        let db = TestResultDatabase::new(None);
        assert!(db.is_ok());
    }

    #[test]
    fn test_record_test_result() {
        let db = TestResultDatabase::new(None).unwrap();

        let id = db
            .record_test_result(
                "unit",
                "test_example",
                TestStatus::Passed,
                100,
                None,
                "1.0.0",
            )
            .unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_record_test_result_with_error() {
        let db = TestResultDatabase::new(None).unwrap();

        let id = db
            .record_test_result(
                "unit",
                "test_failing",
                TestStatus::Failed,
                50,
                Some("assertion failed"),
                "1.0.0",
            )
            .unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_get_test_results() {
        let db = TestResultDatabase::new(None).unwrap();

        db.record_test_result("unit", "test1", TestStatus::Passed, 100, None, "1.0.0")
            .unwrap();
        db.record_test_result("unit", "test2", TestStatus::Failed, 50, None, "1.0.0")
            .unwrap();

        let results = db.get_test_results(Some("unit"), None);
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_get_test_results_by_suite() {
        let db = TestResultDatabase::new(None).unwrap();

        db.record_test_result("unit", "test1", TestStatus::Passed, 100, None, "1.0.0")
            .unwrap();
        db.record_test_result("integration", "test2", TestStatus::Passed, 200, None, "1.0.0")
            .unwrap();

        let unit_results = db.get_test_results(Some("unit"), None);
        assert_eq!(unit_results.len(), 1);
        assert_eq!(unit_results[0].suite, "unit");

        let integration_results = db.get_test_results(Some("integration"), None);
        assert_eq!(integration_results.len(), 1);
        assert_eq!(integration_results[0].suite, "integration");
    }

    #[test]
    fn test_record_metric() {
        let db = TestResultDatabase::new(None).unwrap();

        let id = db
            .record_metric("page_load_time", 1500.0, "ms", Some("https://example.com"))
            .unwrap();

        assert!(id > 0);
    }

    #[test]
    fn test_get_metrics() {
        let db = TestResultDatabase::new(None).unwrap();

        db.record_metric("page_load_time", 1500.0, "ms", Some("https://example.com"))
            .unwrap();
        db.record_metric("memory_usage", 256.0, "MB", None)
            .unwrap();

        let all_metrics = db.get_metrics(None, None);
        assert_eq!(all_metrics.len(), 2);

        let page_load_metrics = db.get_metrics(Some("page_load_time"), None);
        assert_eq!(page_load_metrics.len(), 1);
        assert_eq!(page_load_metrics[0].name, "page_load_time");
    }

    #[test]
    fn test_get_test_summary() {
        let db = TestResultDatabase::new(None).unwrap();

        db.record_test_result("wpt", "test1", TestStatus::Passed, 100, None, "1.0.0")
            .unwrap();
        db.record_test_result("wpt", "test2", TestStatus::Passed, 100, None, "1.0.0")
            .unwrap();
        db.record_test_result("wpt", "test3", TestStatus::Failed, 50, None, "1.0.0")
            .unwrap();
        db.record_test_result("wpt", "test4", TestStatus::Skipped, 0, None, "1.0.0")
            .unwrap();

        let summary = db.get_test_summary("wpt");
        assert_eq!(summary.total, 4);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);
        assert_eq!(summary.skipped, 1);
    }

    #[test]
    fn test_test_summary_pass_rate() {
        let summary = TestSummary {
            total: 10,
            passed: 8,
            failed: 2,
            skipped: 0,
            timeout: 0,
            error: 0,
        };

        assert!((summary.pass_rate() - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_test_summary_pass_rate_empty() {
        let summary = TestSummary::default();
        assert!((summary.pass_rate() - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_test_status_as_str() {
        assert_eq!(TestStatus::Passed.as_str(), "passed");
        assert_eq!(TestStatus::Failed.as_str(), "failed");
        assert_eq!(TestStatus::Skipped.as_str(), "skipped");
        assert_eq!(TestStatus::Timeout.as_str(), "timeout");
        assert_eq!(TestStatus::Error.as_str(), "error");
    }

    #[test]
    fn test_test_status_from_str() {
        assert_eq!(TestStatus::from_str("passed"), Some(TestStatus::Passed));
        assert_eq!(TestStatus::from_str("FAILED"), Some(TestStatus::Failed));
        assert_eq!(TestStatus::from_str("skip"), Some(TestStatus::Skipped));
        assert_eq!(TestStatus::from_str("invalid"), None);
    }

    // ========================================
    // Tests for BrowserMetrics
    // ========================================

    #[test]
    fn test_browser_metrics_new() {
        let metrics = BrowserMetrics::new();
        assert_eq!(metrics.active_tabs, 0);
        assert_eq!(metrics.network_requests, 0);
        assert_eq!(metrics.blocked_requests, 0);
        assert!(metrics.session_start > 0);
    }

    #[test]
    fn test_browser_metrics_record_request() {
        let mut metrics = BrowserMetrics::new();
        metrics.record_request();
        metrics.record_request();
        assert_eq!(metrics.network_requests, 2);
    }

    #[test]
    fn test_browser_metrics_record_blocked() {
        let mut metrics = BrowserMetrics::new();
        metrics.record_request();
        metrics.record_request();
        metrics.record_blocked();
        assert_eq!(metrics.blocked_requests, 1);
    }

    #[test]
    fn test_browser_metrics_record_navigation() {
        let mut metrics = BrowserMetrics::new();
        metrics.record_navigation(1500);
        assert_eq!(metrics.pages_visited, 1);
        assert_eq!(metrics.page_load_time_ms, 1500);

        metrics.record_navigation(2000);
        assert_eq!(metrics.pages_visited, 2);
        assert_eq!(metrics.page_load_time_ms, 2000);
    }

    #[test]
    fn test_browser_metrics_set_active_tabs() {
        let mut metrics = BrowserMetrics::new();
        metrics.set_active_tabs(5);
        assert_eq!(metrics.active_tabs, 5);
    }

    #[test]
    fn test_browser_metrics_set_memory_usage() {
        let mut metrics = BrowserMetrics::new();
        metrics.set_memory_usage(256 * 1024 * 1024); // 256 MB
        assert_eq!(metrics.memory_usage_bytes, 256 * 1024 * 1024);
        assert!((metrics.memory_usage_mb() - 256.0).abs() < 0.01);
    }

    #[test]
    fn test_browser_metrics_block_rate() {
        let mut metrics = BrowserMetrics::new();

        // Empty case
        assert!((metrics.block_rate() - 0.0).abs() < 0.01);

        // With requests
        for _ in 0..10 {
            metrics.record_request();
        }
        for _ in 0..3 {
            metrics.record_blocked();
        }
        assert!((metrics.block_rate() - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_browser_metrics_session_duration() {
        let mut metrics = BrowserMetrics::new();
        std::thread::sleep(std::time::Duration::from_millis(10));
        metrics.record_request(); // Updates timestamp
        assert!(metrics.session_duration_secs() >= 0);
    }

    #[test]
    fn test_browser_metrics_snapshot() {
        let mut metrics = BrowserMetrics::new();
        metrics.set_active_tabs(3);
        metrics.record_request();
        metrics.record_blocked();
        metrics.set_memory_usage(100 * 1024 * 1024);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.active_tabs, 3);
        assert_eq!(snapshot.network_requests, 1);
        assert_eq!(snapshot.blocked_requests, 1);
        assert!((snapshot.block_rate_percent - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_browser_metrics_default() {
        let metrics = BrowserMetrics::default();
        assert_eq!(metrics.active_tabs, 0);
        assert_eq!(metrics.network_requests, 0);
        assert_eq!(metrics.session_start, 0);
    }

    #[test]
    fn test_metrics_snapshot_serialization() {
        let snapshot = MetricsSnapshot {
            active_tabs: 5,
            network_requests: 100,
            blocked_requests: 20,
            block_rate_percent: 20.0,
            memory_usage_mb: 256.0,
            page_load_time_ms: 1500,
            pages_visited: 10,
            session_duration_secs: 3600,
        };

        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("active_tabs"));
        assert!(json.contains("100"));

        let deserialized: MetricsSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.active_tabs, 5);
        assert_eq!(deserialized.network_requests, 100);
    }
}
