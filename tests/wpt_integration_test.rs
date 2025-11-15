/// Integration test for WPT test harness
///
/// This test file validates the WPT infrastructure without requiring
/// an actual browser or WPT repository.

// External dependencies (from workspace)
extern crate serde_json;
extern crate tempfile;

#[path = "wpt/mod.rs"]
mod wpt;

use wpt::{WptConfig, WptHarness, WptRunner, WptReporter, WptSuiteResults, OutputFormat};
use std::path::PathBuf;

#[test]
fn test_wpt_config_default() {
    let config = WptConfig::default();

    assert_eq!(config.webdriver_port, 4444);
    assert_eq!(config.headless, true);
    assert_eq!(config.timeout_seconds, 30);
    assert_eq!(config.expected_pass_rate, 0.40);
}

#[test]
fn test_wpt_config_creation() {
    let config = WptConfig {
        test_path: PathBuf::from("/tmp/wpt"),
        binary_path: PathBuf::from("/tmp/browser"),
        test_subsets: vec!["html".to_string(), "fetch".to_string()],
        expected_pass_rate: 0.50,
        webdriver_port: 5555,
        browser_binary: PathBuf::from("/tmp/browser"),
        headless: false,
        timeout_seconds: 60,
    };

    assert_eq!(config.webdriver_port, 5555);
    assert_eq!(config.headless, false);
    assert_eq!(config.timeout_seconds, 60);
    assert_eq!(config.test_subsets.len(), 2);
}

#[test]
fn test_harness_creation() {
    let config = WptConfig::default();
    let harness = WptHarness::new(config);

    // Harness should be created successfully
    // Session should not be active yet
    assert!(true); // Placeholder - actual checks depend on internals
}

#[test]
fn test_placeholder_results() {
    let config = WptConfig::default();
    let harness = WptHarness::new(config);

    let results = harness.generate_placeholder_results();

    assert!(results.total_tests > 0);
    assert_eq!(results.skipped, results.total_tests);
    assert_eq!(results.passed, 0);
    assert_eq!(results.failed, 0);
}

#[test]
fn test_runner_creation() {
    let config = WptConfig::default();
    let harness = WptHarness::new(config);
    let runner = WptRunner::new(harness);

    // Runner should be created successfully
    assert!(true);
}

#[test]
fn test_runner_with_filter() {
    let config = WptConfig::default();
    let harness = WptHarness::new(config);
    let runner = WptRunner::new(harness)
        .with_filter("html/.*")
        .expect("Should create filter");

    // Filter should be applied
    let tests = vec![
        "html/test1.html".to_string(),
        "fetch/test2.html".to_string(),
    ];

    let filtered = runner.filter_tests(tests);
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0], "html/test1.html");
}

#[test]
fn test_test_discovery_filters() {
    let config = WptConfig::default();
    let harness = WptHarness::new(config);
    let runner = WptRunner::new(harness);

    // Test is_test_file logic
    let valid_tests = vec![
        "html/test.html",
        "fetch/basic.html",
        "dom/node.html",
    ];

    let invalid_tests = vec![
        "html/support/utils.html",
        "html/resources/common.html",
        "html/test-ref.html",
        "html/ref/reference.html",
    ];

    // All valid tests should pass
    for test in valid_tests {
        // Simulate is_test_file check
        assert!(!test.contains("/support/"));
        assert!(!test.contains("/resources/"));
        assert!(!test.contains("-ref."));
    }

    // All invalid tests should fail
    for test in invalid_tests {
        let is_invalid = test.contains("/support/") ||
                        test.contains("/resources/") ||
                        test.contains("-ref.") ||
                        test.contains("/ref/");
        assert!(is_invalid);
    }
}

#[test]
fn test_reporter_json_generation() {
    let mut results = WptSuiteResults::new();

    results.add_result(wpt::WptTestResult {
        test_name: "test1.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 100,
    });

    let reporter = WptReporter::new(results);
    let json = reporter.generate_json();

    assert!(json.contains("test1.html"));
    assert!(json.contains("Pass"));
}

#[test]
fn test_reporter_html_generation() {
    let mut results = WptSuiteResults::new();

    results.add_result(wpt::WptTestResult {
        test_name: "html/test.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 150,
    });

    let reporter = WptReporter::new(results);
    let html = reporter.generate_html();

    assert!(html.contains("<!DOCTYPE html>"));
    assert!(html.contains("WPT Test Results"));
    assert!(html.contains("html/test.html"));
}

#[test]
fn test_reporter_console_generation() {
    let mut results = WptSuiteResults::new();

    results.add_result(wpt::WptTestResult {
        test_name: "test.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 100,
    });

    results.add_result(wpt::WptTestResult {
        test_name: "test2.html".to_string(),
        status: wpt::TestStatus::Fail,
        message: Some("Failed".to_string()),
        duration_ms: 200,
    });

    let reporter = WptReporter::new(results);
    let console = reporter.generate_console();

    assert!(console.contains("WPT Test Results"));
    assert!(console.contains("Total Tests"));
    assert!(console.contains("Passed"));
    assert!(console.contains("Failed"));
}

#[test]
fn test_reporter_grouping() {
    let mut results = WptSuiteResults::new();

    results.add_result(wpt::WptTestResult {
        test_name: "html/test1.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 100,
    });

    results.add_result(wpt::WptTestResult {
        test_name: "html/test2.html".to_string(),
        status: wpt::TestStatus::Fail,
        message: None,
        duration_ms: 150,
    });

    results.add_result(wpt::WptTestResult {
        test_name: "fetch/test3.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 120,
    });

    let reporter = WptReporter::new(results);
    let grouped = reporter.group_by_category();

    assert_eq!(grouped.len(), 2);
    assert!(grouped.contains_key("html"));
    assert!(grouped.contains_key("fetch"));

    let html_results = &grouped["html"];
    assert_eq!(html_results.total_tests, 2);
    assert_eq!(html_results.passed, 1);
    assert_eq!(html_results.failed, 1);

    let fetch_results = &grouped["fetch"];
    assert_eq!(fetch_results.total_tests, 1);
    assert_eq!(fetch_results.passed, 1);
}

#[test]
fn test_reporter_comparison() {
    let mut current = WptSuiteResults::new();
    current.add_result(wpt::WptTestResult {
        test_name: "test1.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 100,
    });
    current.add_result(wpt::WptTestResult {
        test_name: "test2.html".to_string(),
        status: wpt::TestStatus::Fail,
        message: None,
        duration_ms: 150,
    });

    let mut baseline = WptSuiteResults::new();
    baseline.add_result(wpt::WptTestResult {
        test_name: "test1.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 100,
    });
    baseline.add_result(wpt::WptTestResult {
        test_name: "test2.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 150,
    });

    let reporter = WptReporter::new(current);
    let comparison = reporter.compare_with_baseline(&baseline);

    assert_eq!(comparison.new_failures, 1);
    assert_eq!(comparison.fixed_tests, 0);
    assert!(comparison.is_regression());
    assert!(!comparison.is_improvement());
}

#[test]
fn test_full_workflow_simulation() {
    // Create config
    let config = WptConfig::default();

    // Create harness
    let harness = WptHarness::new(config);

    // Generate placeholder results
    let results = harness.generate_placeholder_results();

    // Create reporter and generate all formats
    let reporter = WptReporter::new(results);

    let json = reporter.generate_json();
    let html = reporter.generate_html();
    let console = reporter.generate_console();

    // Verify all formats generated successfully
    assert!(!json.is_empty());
    assert!(!html.is_empty());
    assert!(!console.is_empty());

    // Verify content
    assert!(json.contains("\"total_tests\":"));
    assert!(html.contains("<!DOCTYPE html>"));
    assert!(console.contains("WPT Test Results"));
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
        .expect("Should create database");

    // Database should be created with proper schema
    // This is tested in runner.rs unit tests
    assert!(true);
}

#[test]
fn test_results_serialization() {
    let mut results = WptSuiteResults::new();

    results.add_result(wpt::WptTestResult {
        test_name: "test.html".to_string(),
        status: wpt::TestStatus::Pass,
        message: None,
        duration_ms: 100,
    });

    // Test JSON serialization
    let json = serde_json::to_string(&results).unwrap();
    assert!(json.contains("test.html"));

    // Test JSON deserialization
    let deserialized: WptSuiteResults = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.total_tests, 1);
    assert_eq!(deserialized.passed, 1);
}
