/// WPT Test Runner
///
/// Discovers WPT tests, filters them, executes them via WebDriver harness,
/// and saves results to database for tracking over time.

use super::harness::{WptConfig, WptHarness, WptSuiteResults, WptTestResult, TestStatus};
use std::path::{Path, PathBuf};

// External dependencies (from workspace)
extern crate serde;
extern crate regex;
extern crate rusqlite;
extern crate walkdir;

use regex::Regex;
use rusqlite::{Connection, params};
use walkdir::WalkDir;

/// WPT Test Runner with discovery and execution logic
pub struct WptRunner {
    harness: WptHarness,
    database: Option<Connection>,
    filter: Option<Regex>,
}

impl WptRunner {
    /// Create a new test runner with the given harness
    pub fn new(harness: WptHarness) -> Self {
        Self {
            harness,
            database: None,
            filter: None,
        }
    }

    /// Add database connection for result tracking
    pub fn with_database(mut self, db_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let conn = Connection::open(db_path)?;

        // Create schema if not exists
        conn.execute(
            "CREATE TABLE IF NOT EXISTS test_runs (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                test_suite TEXT NOT NULL,
                total_tests INTEGER NOT NULL,
                passed INTEGER NOT NULL,
                failed INTEGER NOT NULL,
                timeout INTEGER NOT NULL,
                skipped INTEGER NOT NULL,
                errors INTEGER NOT NULL,
                pass_rate REAL NOT NULL,
                commit_hash TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
            [],
        )?;

        conn.execute(
            "CREATE TABLE IF NOT EXISTS test_results (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                run_id INTEGER NOT NULL,
                test_name TEXT NOT NULL,
                status TEXT NOT NULL,
                duration_ms INTEGER NOT NULL,
                message TEXT,
                FOREIGN KEY(run_id) REFERENCES test_runs(id)
            )",
            [],
        )?;

        self.database = Some(conn);
        Ok(self)
    }

    /// Add a regex filter for test selection
    pub fn with_filter(mut self, pattern: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let regex = Regex::new(pattern)?;
        self.filter = Some(regex);
        Ok(self)
    }

    /// Discover all WPT test files in the configured path
    pub fn discover_tests(&self, wpt_path: &Path) -> Vec<String> {
        let mut tests = Vec::new();

        if !wpt_path.exists() {
            eprintln!("Warning: WPT path does not exist: {:?}", wpt_path);
            return tests;
        }

        // Walk directory tree looking for test files
        for entry in WalkDir::new(wpt_path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Check if file is a test file (html, htm, xhtml)
            if let Some(ext) = path.extension() {
                let ext_str = ext.to_string_lossy();
                if ext_str == "html" || ext_str == "htm" || ext_str == "xhtml" {
                    // Get relative path from wpt_path
                    if let Ok(rel_path) = path.strip_prefix(wpt_path) {
                        let test_path = rel_path.to_string_lossy().to_string();

                        // Skip non-test files
                        if !self.is_test_file(&test_path) {
                            continue;
                        }

                        tests.push(test_path);
                    }
                }
            }
        }

        tests.sort();
        tests
    }

    /// Check if a file is a test file (not support file)
    fn is_test_file(&self, path: &str) -> bool {
        // Skip support directories
        if path.contains("/support/") || path.contains("/resources/") {
            return false;
        }

        // Skip reference files
        if path.contains("-ref.") || path.contains("/ref/") {
            return false;
        }

        // Skip test harness files
        if path.contains("testharness") && !path.contains(".any.") {
            return false;
        }

        true
    }

    /// Filter tests based on configured filter and subsets
    pub fn filter_tests(&self, tests: Vec<String>) -> Vec<String> {
        let config = &self.harness.config;

        tests
            .into_iter()
            .filter(|test| {
                // Filter by test_subsets if configured
                if !config.test_subsets.is_empty() {
                    let matches_subset = config.test_subsets.iter().any(|subset| {
                        test.starts_with(subset)
                    });
                    if !matches_subset {
                        return false;
                    }
                }

                // Filter by regex pattern if configured
                if let Some(ref filter) = self.filter {
                    if !filter.is_match(test) {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    /// Execute a single test file
    pub fn execute_test_file(&self, path: &str) -> Result<WptTestResult, Box<dyn std::error::Error>> {
        self.harness.run_test(path)
    }

    /// Parse test result from test harness output
    pub fn parse_test_result(output: &str) -> WptTestResult {
        // This is a placeholder - actual implementation would parse
        // WPT testharness.js output format
        // For now, return a basic result
        use super::harness::TestStatus;

        WptTestResult {
            test_name: "unknown".to_string(),
            status: if output.contains("PASS") {
                TestStatus::Pass
            } else if output.contains("FAIL") {
                TestStatus::Fail
            } else if output.contains("TIMEOUT") {
                TestStatus::Timeout
            } else {
                TestStatus::Error
            },
            message: Some(output.to_string()),
            duration_ms: 0,
        }
    }

    /// Run all discovered and filtered tests
    pub fn run_all(&mut self) -> Result<WptSuiteResults, Box<dyn std::error::Error>> {
        // Discover tests
        let all_tests = self.discover_tests(&self.harness.config.test_path.clone());
        println!("Discovered {} tests", all_tests.len());

        // Filter tests
        let filtered_tests = self.filter_tests(all_tests);
        println!("Running {} tests after filtering", filtered_tests.len());

        // Start browser and create session
        self.harness.start_browser()?;
        self.harness.create_session()?;

        // Run tests
        let results = self.harness.run_test_suite(&filtered_tests)?;

        // Save results to database if configured
        if self.database.is_some() {
            self.save_results(&results)?;
        }

        Ok(results)
    }

    /// Save test results to database
    pub fn save_results(&mut self, results: &WptSuiteResults) -> Result<(), Box<dyn std::error::Error>> {
        let conn = self.database.as_ref()
            .ok_or("No database connection")?;

        // Get current git commit hash (if available)
        let commit_hash = Self::get_git_commit_hash();

        // Insert test run
        conn.execute(
            "INSERT INTO test_runs (
                test_suite, total_tests, passed, failed, timeout, skipped, errors, pass_rate, commit_hash
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                "wpt",
                results.total_tests,
                results.passed,
                results.failed,
                results.timeout,
                results.skipped,
                results.errors,
                results.pass_rate,
                commit_hash,
            ],
        )?;

        let run_id = conn.last_insert_rowid();

        // Insert individual test results
        for result in &results.results {
            conn.execute(
                "INSERT INTO test_results (
                    run_id, test_name, status, duration_ms, message
                ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    run_id,
                    result.test_name,
                    result.status.as_str(),
                    result.duration_ms as i64,
                    result.message,
                ],
            )?;
        }

        println!("Saved results to database (run_id: {})", run_id);
        Ok(())
    }

    /// Get current git commit hash
    fn get_git_commit_hash() -> Option<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .ok()?;

        if output.status.success() {
            Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            None
        }
    }

    /// Get results statistics from database
    pub fn get_statistics(&self, limit: usize) -> Result<Vec<RunStatistics>, Box<dyn std::error::Error>> {
        let conn = self.database.as_ref()
            .ok_or("No database connection")?;

        let mut stmt = conn.prepare(
            "SELECT id, test_suite, total_tests, passed, failed, timeout, skipped, errors,
                    pass_rate, commit_hash, timestamp
             FROM test_runs
             ORDER BY timestamp DESC
             LIMIT ?1"
        )?;

        let stats = stmt.query_map([limit], |row| {
            Ok(RunStatistics {
                run_id: row.get(0)?,
                test_suite: row.get(1)?,
                total_tests: row.get(2)?,
                passed: row.get(3)?,
                failed: row.get(4)?,
                timeout: row.get(5)?,
                skipped: row.get(6)?,
                errors: row.get(7)?,
                pass_rate: row.get(8)?,
                commit_hash: row.get(9)?,
                timestamp: row.get(10)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

        Ok(stats)
    }
}

/// Statistics for a test run
#[derive(Debug, Clone)]
pub struct RunStatistics {
    pub run_id: i64,
    pub test_suite: String,
    pub total_tests: usize,
    pub passed: usize,
    pub failed: usize,
    pub timeout: usize,
    pub skipped: usize,
    pub errors: usize,
    pub pass_rate: f64,
    pub commit_hash: Option<String>,
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runner_creation() {
        let config = WptConfig::default();
        let harness = WptHarness::new(config);
        let runner = WptRunner::new(harness);

        assert!(runner.database.is_none());
        assert!(runner.filter.is_none());
    }

    #[test]
    fn test_filter_addition() {
        let config = WptConfig::default();
        let harness = WptHarness::new(config);
        let runner = WptRunner::new(harness)
            .with_filter("html/.*")
            .unwrap();

        assert!(runner.filter.is_some());
    }

    #[test]
    fn test_is_test_file() {
        let config = WptConfig::default();
        let harness = WptHarness::new(config);
        let runner = WptRunner::new(harness);

        // Valid test files
        assert!(runner.is_test_file("html/test.html"));
        assert!(runner.is_test_file("fetch/basic.html"));

        // Invalid test files
        assert!(!runner.is_test_file("html/support/utils.html"));
        assert!(!runner.is_test_file("html/resources/common.html"));
        assert!(!runner.is_test_file("html/test-ref.html"));
    }

    #[test]
    fn test_filter_tests() {
        let mut config = WptConfig::default();
        config.test_subsets = vec!["html".to_string()];

        let harness = WptHarness::new(config);
        let runner = WptRunner::new(harness);

        let all_tests = vec![
            "html/test1.html".to_string(),
            "html/test2.html".to_string(),
            "fetch/test3.html".to_string(),
        ];

        let filtered = runner.filter_tests(all_tests);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"html/test1.html".to_string()));
        assert!(filtered.contains(&"html/test2.html".to_string()));
    }

    #[test]
    fn test_filter_tests_with_regex() {
        let config = WptConfig::default();
        let harness = WptHarness::new(config);
        let runner = WptRunner::new(harness)
            .with_filter(".*test1.*")
            .unwrap();

        let all_tests = vec![
            "html/test1.html".to_string(),
            "html/test2.html".to_string(),
            "fetch/test1.html".to_string(),
        ];

        let filtered = runner.filter_tests(all_tests);

        assert_eq!(filtered.len(), 2);
        assert!(filtered.contains(&"html/test1.html".to_string()));
        assert!(filtered.contains(&"fetch/test1.html".to_string()));
    }

    #[test]
    fn test_parse_test_result() {
        let output_pass = "PASS: Test passed";
        let result = WptRunner::parse_test_result(output_pass);
        assert_eq!(result.status, TestStatus::Pass);

        let output_fail = "FAIL: Test failed";
        let result = WptRunner::parse_test_result(output_fail);
        assert_eq!(result.status, TestStatus::Fail);

        let output_timeout = "TIMEOUT: Test timed out";
        let result = WptRunner::parse_test_result(output_timeout);
        assert_eq!(result.status, TestStatus::Timeout);
    }

    #[test]
    fn test_database_schema_creation() {
        use tempfile::NamedTempFile;

        let temp_file = NamedTempFile::new().unwrap();
        let db_path = temp_file.path().to_str().unwrap();

        let config = WptConfig::default();
        let harness = WptHarness::new(config);
        let runner = WptRunner::new(harness)
            .with_database(db_path)
            .unwrap();

        assert!(runner.database.is_some());
    }
}
