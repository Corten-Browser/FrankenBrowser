/// WPT Test Results Reporter
///
/// Generates reports in multiple formats: JSON, HTML, and Console

use super::harness::{WptSuiteResults, WptTestResult, TestStatus};
use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::Path;

// External dependencies (from workspace)
extern crate serde;
extern crate serde_json;

/// Output format for test reports
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Html,
    Console,
}

/// WPT Test Results Reporter
pub struct WptReporter {
    results: WptSuiteResults,
}

impl WptReporter {
    /// Create a new reporter with test results
    pub fn new(results: WptSuiteResults) -> Self {
        Self { results }
    }

    /// Generate JSON report
    pub fn generate_json(&self) -> String {
        serde_json::to_string_pretty(&self.results)
            .unwrap_or_else(|e| format!(r#"{{"error": "Failed to serialize: {}"}}"#, e))
    }

    /// Generate HTML dashboard report
    pub fn generate_html(&self) -> String {
        let pass_rate_pct = self.results.pass_rate * 100.0;
        let pass_class = if pass_rate_pct >= 80.0 {
            "success"
        } else if pass_rate_pct >= 50.0 {
            "warning"
        } else {
            "danger"
        };

        // Group results by category (top-level directory)
        let grouped = self.group_by_category();

        let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>WPT Test Results - FrankenBrowser</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            margin: 0;
            padding: 20px;
            background: #f5f5f5;
        }
        .container {
            max-width: 1200px;
            margin: 0 auto;
            background: white;
            padding: 30px;
            border-radius: 8px;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #007bff;
            padding-bottom: 10px;
        }
        .summary {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
            gap: 20px;
            margin: 30px 0;
        }
        .metric {
            background: #f8f9fa;
            padding: 20px;
            border-radius: 6px;
            text-align: center;
        }
        .metric-value {
            font-size: 36px;
            font-weight: bold;
            margin: 10px 0;
        }
        .metric-label {
            color: #666;
            font-size: 14px;
        }
        .success { color: #28a745; }
        .warning { color: #ffc107; }
        .danger { color: #dc3545; }
        .info { color: #17a2b8; }

        table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 20px;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #dee2e6;
        }
        th {
            background: #f8f9fa;
            font-weight: 600;
            color: #495057;
        }
        tr:hover {
            background: #f8f9fa;
        }
        .status-badge {
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 12px;
            font-weight: 600;
            text-transform: uppercase;
        }
        .status-pass { background: #d4edda; color: #155724; }
        .status-fail { background: #f8d7da; color: #721c24; }
        .status-timeout { background: #fff3cd; color: #856404; }
        .status-skip { background: #d1ecf1; color: #0c5460; }
        .status-error { background: #f8d7da; color: #721c24; }

        .filter-box {
            margin: 20px 0;
            padding: 15px;
            background: #f8f9fa;
            border-radius: 6px;
        }
        #searchInput {
            width: 100%;
            padding: 10px;
            border: 1px solid #ced4da;
            border-radius: 4px;
            font-size: 14px;
        }
        .category-section {
            margin-top: 30px;
        }
        .category-header {
            background: #007bff;
            color: white;
            padding: 10px 15px;
            border-radius: 6px;
            margin-top: 20px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🧪 WPT Test Results - FrankenBrowser</h1>

        <div class="summary">
            <div class="metric">
                <div class="metric-label">Total Tests</div>
                <div class="metric-value info">"#);

        html.push_str(&format!("{}", self.results.total_tests));
        html.push_str(r#"</div>
            </div>
            <div class="metric">
                <div class="metric-label">Passed</div>
                <div class="metric-value success">"#);
        html.push_str(&format!("{}", self.results.passed));
        html.push_str(r#"</div>
            </div>
            <div class="metric">
                <div class="metric-label">Failed</div>
                <div class="metric-value danger">"#);
        html.push_str(&format!("{}", self.results.failed));
        html.push_str(r#"</div>
            </div>
            <div class="metric">
                <div class="metric-label">Pass Rate</div>
                <div class="metric-value "#);
        html.push_str(pass_class);
        html.push_str(r#"">"#);
        html.push_str(&format!("{:.1}%", pass_rate_pct));
        html.push_str(r#"</div>
            </div>
        </div>

        <div class="filter-box">
            <input type="text" id="searchInput" placeholder="Filter tests by name..." onkeyup="filterTests()">
        </div>
        "#);

        // Add category sections
        for (category, cat_results) in grouped.iter() {
            let cat_pass_rate = cat_results.pass_rate * 100.0;
            html.push_str(&format!(
                r#"<div class="category-section">
                <h3 class="category-header">{} ({}/{} tests, {:.1}% pass rate)</h3>
                <table id="testTable-{}">
                    <thead>
                        <tr>
                            <th>Test Name</th>
                            <th>Status</th>
                            <th>Duration (ms)</th>
                            <th>Message</th>
                        </tr>
                    </thead>
                    <tbody>
"#,
                category,
                cat_results.passed,
                cat_results.total_tests,
                cat_pass_rate,
                category.replace("/", "-")
            ));

            for result in &cat_results.results {
                let status_class = match result.status {
                    TestStatus::Pass => "status-pass",
                    TestStatus::Fail => "status-fail",
                    TestStatus::Timeout => "status-timeout",
                    TestStatus::Skip => "status-skip",
                    TestStatus::Error => "status-error",
                };

                html.push_str(&format!(
                    r#"<tr class="test-row">
                        <td>{}</td>
                        <td><span class="status-badge {}">{}</span></td>
                        <td>{}</td>
                        <td>{}</td>
                    </tr>
"#,
                    Self::html_escape(&result.test_name),
                    status_class,
                    result.status.as_str(),
                    result.duration_ms,
                    Self::html_escape(&result.message.clone().unwrap_or_default())
                ));
            }

            html.push_str(r#"</tbody>
                </table>
            </div>
"#);
        }

        html.push_str(r#"
    </div>

    <script>
        function filterTests() {
            const input = document.getElementById('searchInput');
            const filter = input.value.toLowerCase();
            const rows = document.querySelectorAll('.test-row');

            rows.forEach(row => {
                const text = row.textContent.toLowerCase();
                row.style.display = text.includes(filter) ? '' : 'none';
            });
        }
    </script>
</body>
</html>
"#);

        html
    }

    /// Generate console output
    pub fn generate_console(&self) -> String {
        let mut output = String::new();

        output.push_str("\n");
        output.push_str("╔════════════════════════════════════════════════════════════╗\n");
        output.push_str("║           WPT Test Results - FrankenBrowser               ║\n");
        output.push_str("╠════════════════════════════════════════════════════════════╣\n");

        output.push_str(&format!("║ Total Tests:    {:>43} ║\n", self.results.total_tests));
        output.push_str(&format!("║ Passed:         {:>43} ║\n", format!("{} ✅", self.results.passed)));
        output.push_str(&format!("║ Failed:         {:>43} ║\n", format!("{} ❌", self.results.failed)));
        output.push_str(&format!("║ Timeout:        {:>43} ║\n", format!("{} ⏱️", self.results.timeout)));
        output.push_str(&format!("║ Skipped:        {:>43} ║\n", format!("{} ⏭️", self.results.skipped)));
        output.push_str(&format!("║ Errors:         {:>43} ║\n", format!("{} ⚠️", self.results.errors)));
        output.push_str(&format!("║ Pass Rate:      {:>43} ║\n", format!("{:.1}%", self.results.pass_rate * 100.0)));

        output.push_str("╚════════════════════════════════════════════════════════════╝\n");
        output.push_str("\n");

        // Group by category and show summary
        let grouped = self.group_by_category();
        if !grouped.is_empty() {
            output.push_str("Category Breakdown:\n");
            output.push_str("─────────────────────────────────────────────────────────────\n");

            for (category, results) in grouped.iter() {
                output.push_str(&format!(
                    "  {:<30} {:>4}/{:<4} tests ({:.1}% pass)\n",
                    category,
                    results.passed,
                    results.total_tests,
                    results.pass_rate * 100.0
                ));
            }
            output.push_str("\n");
        }

        // Show failed tests
        let failed_tests: Vec<&WptTestResult> = self.results.results.iter()
            .filter(|r| r.status == TestStatus::Fail || r.status == TestStatus::Error)
            .collect();

        if !failed_tests.is_empty() {
            output.push_str("Failed Tests:\n");
            output.push_str("─────────────────────────────────────────────────────────────\n");

            for result in failed_tests.iter().take(20) {
                output.push_str(&format!("  ❌ {}\n", result.test_name));
                if let Some(ref msg) = result.message {
                    output.push_str(&format!("     {}\n", msg));
                }
            }

            if failed_tests.len() > 20 {
                output.push_str(&format!("  ... and {} more\n", failed_tests.len() - 20));
            }
            output.push_str("\n");
        }

        output
    }

    /// Write report to file
    pub fn write_to_file(&self, path: &Path, format: OutputFormat) -> Result<(), Box<dyn std::error::Error>> {
        let content = match format {
            OutputFormat::Json => self.generate_json(),
            OutputFormat::Html => self.generate_html(),
            OutputFormat::Console => self.generate_console(),
        };

        let mut file = File::create(path)?;
        file.write_all(content.as_bytes())?;

        Ok(())
    }

    /// Calculate overall pass rate
    pub fn calculate_pass_rate(&self) -> f64 {
        self.results.pass_rate
    }

    /// Group results by category (top-level directory)
    pub fn group_by_category(&self) -> HashMap<String, WptSuiteResults> {
        let mut grouped: HashMap<String, WptSuiteResults> = HashMap::new();

        for result in &self.results.results {
            let category = result.test_name
                .split('/')
                .next()
                .unwrap_or("other")
                .to_string();

            let suite = grouped.entry(category).or_insert_with(WptSuiteResults::new);
            suite.add_result(result.clone());
        }

        grouped
    }

    /// Compare with baseline results
    pub fn compare_with_baseline(&self, baseline: &WptSuiteResults) -> Comparison {
        Comparison {
            current_pass_rate: self.results.pass_rate,
            baseline_pass_rate: baseline.pass_rate,
            pass_rate_delta: self.results.pass_rate - baseline.pass_rate,
            tests_added: self.results.total_tests as i32 - baseline.total_tests as i32,
            new_failures: self.count_new_failures(baseline),
            fixed_tests: self.count_fixed_tests(baseline),
        }
    }

    /// Count tests that newly failed compared to baseline
    fn count_new_failures(&self, baseline: &WptSuiteResults) -> usize {
        let baseline_map: HashMap<&str, &WptTestResult> = baseline.results.iter()
            .map(|r| (r.test_name.as_str(), r))
            .collect();

        self.results.results.iter()
            .filter(|r| {
                r.status == TestStatus::Fail || r.status == TestStatus::Error
            })
            .filter(|r| {
                // Check if this test passed in baseline
                baseline_map.get(r.test_name.as_str())
                    .map(|baseline_result| baseline_result.status == TestStatus::Pass)
                    .unwrap_or(false)
            })
            .count()
    }

    /// Count tests that were fixed compared to baseline
    fn count_fixed_tests(&self, baseline: &WptSuiteResults) -> usize {
        let baseline_map: HashMap<&str, &WptTestResult> = baseline.results.iter()
            .map(|r| (r.test_name.as_str(), r))
            .collect();

        self.results.results.iter()
            .filter(|r| r.status == TestStatus::Pass)
            .filter(|r| {
                // Check if this test failed in baseline
                baseline_map.get(r.test_name.as_str())
                    .map(|baseline_result| {
                        baseline_result.status == TestStatus::Fail ||
                        baseline_result.status == TestStatus::Error
                    })
                    .unwrap_or(false)
            })
            .count()
    }

    /// HTML escape helper
    fn html_escape(s: &str) -> String {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
            .replace('\'', "&#39;")
    }
}

/// Comparison between current and baseline results
#[derive(Debug, Clone)]
pub struct Comparison {
    pub current_pass_rate: f64,
    pub baseline_pass_rate: f64,
    pub pass_rate_delta: f64,
    pub tests_added: i32,
    pub new_failures: usize,
    pub fixed_tests: usize,
}

impl Comparison {
    /// Check if this represents a regression
    pub fn is_regression(&self) -> bool {
        self.pass_rate_delta < -0.01 || self.new_failures > 0
    }

    /// Check if this represents an improvement
    pub fn is_improvement(&self) -> bool {
        self.pass_rate_delta > 0.01 || self.fixed_tests > 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_results() -> WptSuiteResults {
        let mut results = WptSuiteResults::new();

        results.add_result(WptTestResult {
            test_name: "html/test1.html".to_string(),
            status: TestStatus::Pass,
            message: None,
            duration_ms: 100,
        });

        results.add_result(WptTestResult {
            test_name: "html/test2.html".to_string(),
            status: TestStatus::Fail,
            message: Some("Assertion failed".to_string()),
            duration_ms: 150,
        });

        results.add_result(WptTestResult {
            test_name: "fetch/test3.html".to_string(),
            status: TestStatus::Pass,
            message: None,
            duration_ms: 120,
        });

        results
    }

    #[test]
    fn test_reporter_creation() {
        let results = create_test_results();
        let reporter = WptReporter::new(results);

        assert_eq!(reporter.calculate_pass_rate(), 2.0 / 3.0);
    }

    #[test]
    fn test_json_generation() {
        let results = create_test_results();
        let reporter = WptReporter::new(results);

        let json = reporter.generate_json();
        assert!(json.contains("html/test1.html"));
        assert!(json.contains("Pass"));
        assert!(json.contains("Fail"));
    }

    #[test]
    fn test_html_generation() {
        let results = create_test_results();
        let reporter = WptReporter::new(results);

        let html = reporter.generate_html();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("html/test1.html"));
        assert!(html.contains("WPT Test Results"));
    }

    #[test]
    fn test_console_generation() {
        let results = create_test_results();
        let reporter = WptReporter::new(results);

        let console = reporter.generate_console();
        assert!(console.contains("WPT Test Results"));
        assert!(console.contains("Total Tests"));
        assert!(console.contains("Passed"));
    }

    #[test]
    fn test_group_by_category() {
        let results = create_test_results();
        let reporter = WptReporter::new(results);

        let grouped = reporter.group_by_category();

        assert_eq!(grouped.len(), 2);
        assert!(grouped.contains_key("html"));
        assert!(grouped.contains_key("fetch"));

        assert_eq!(grouped.get("html").unwrap().total_tests, 2);
        assert_eq!(grouped.get("fetch").unwrap().total_tests, 1);
    }

    #[test]
    fn test_comparison() {
        let current = create_test_results();
        let mut baseline = WptSuiteResults::new();

        baseline.add_result(WptTestResult {
            test_name: "html/test1.html".to_string(),
            status: TestStatus::Pass,
            message: None,
            duration_ms: 100,
        });

        baseline.add_result(WptTestResult {
            test_name: "html/test2.html".to_string(),
            status: TestStatus::Pass,
            message: None,
            duration_ms: 150,
        });

        let reporter = WptReporter::new(current);
        let comparison = reporter.compare_with_baseline(&baseline);

        assert_eq!(comparison.new_failures, 1);
        assert!(comparison.is_regression());
    }

    #[test]
    fn test_html_escape() {
        assert_eq!(
            WptReporter::html_escape("<script>alert('xss')</script>"),
            "&lt;script&gt;alert(&#39;xss&#39;)&lt;/script&gt;"
        );
    }
}
