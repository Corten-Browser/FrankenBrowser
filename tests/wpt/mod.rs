/// WPT Test Harness Module
///
/// This module provides infrastructure for running Web Platform Tests (WPT)
/// against FrankenBrowser using WebDriver protocol.

pub mod harness;
pub mod runner;
pub mod reporter;

// Re-export commonly used types
pub use harness::{
    WptConfig, WptHarness, WptSuiteResults, WptTestResult, TestStatus,
};
pub use runner::{WptRunner, RunStatistics};
pub use reporter::{WptReporter, OutputFormat, Comparison};
