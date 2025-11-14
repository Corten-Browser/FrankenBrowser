use crate::test_recorder::{TestRecorder, TestRun, BenchmarkResult, QualityMetric};
use rusqlite::Result;
use std::fs;
use std::path::Path;

/// Dashboard generator for test results
pub struct DashboardGenerator {
    recorder: TestRecorder,
}

impl DashboardGenerator {
    /// Create a new dashboard generator
    pub fn new(recorder: TestRecorder) -> Self {
        Self { recorder }
    }

    /// Generate complete HTML dashboard
    pub fn generate_html_dashboard<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let latest_results = self.recorder.get_latest_results()?;
        let quality_summary = self.recorder.get_quality_summary()?;

        let html = self.build_html(&latest_results, &quality_summary);

        fs::write(output_path, html)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        Ok(())
    }

    /// Generate Markdown dashboard
    pub fn generate_markdown_dashboard<P: AsRef<Path>>(&self, output_path: P) -> Result<()> {
        let latest_results = self.recorder.get_latest_results()?;
        let quality_summary = self.recorder.get_quality_summary()?;

        let markdown = self.build_markdown(&latest_results, &quality_summary);

        fs::write(output_path, markdown)
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        Ok(())
    }

    fn build_html(&self, test_results: &[TestRun], quality_metrics: &[QualityMetric]) -> String {
        let mut html = String::from(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>FrankenBrowser Test Dashboard</title>
    <style>
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Oxygen, Ubuntu, Cantarell, sans-serif;
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
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        h1 {
            color: #333;
            border-bottom: 3px solid #4CAF50;
            padding-bottom: 10px;
        }
        h2 {
            color: #555;
            margin-top: 30px;
        }
        table {
            width: 100%;
            border-collapse: collapse;
            margin: 20px 0;
        }
        th, td {
            padding: 12px;
            text-align: left;
            border-bottom: 1px solid #ddd;
        }
        th {
            background: #f8f8f8;
            font-weight: 600;
            color: #333;
        }
        .pass { color: #4CAF50; font-weight: bold; }
        .fail { color: #f44336; font-weight: bold; }
        .warning { color: #ff9800; font-weight: bold; }
        .badge {
            display: inline-block;
            padding: 4px 8px;
            border-radius: 4px;
            font-size: 0.85em;
            font-weight: bold;
        }
        .badge-success { background: #4CAF50; color: white; }
        .badge-danger { background: #f44336; color: white; }
        .badge-warning { background: #ff9800; color: white; }
        .metric-card {
            display: inline-block;
            padding: 20px;
            margin: 10px;
            background: #f8f8f8;
            border-radius: 8px;
            min-width: 200px;
        }
        .metric-value {
            font-size: 2em;
            font-weight: bold;
            color: #4CAF50;
        }
        .metric-label {
            color: #777;
            margin-top: 5px;
        }
        .timestamp {
            color: #999;
            font-size: 0.9em;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>🧪 FrankenBrowser Test Dashboard</h1>
        <p class="timestamp">Generated: "#);

        html.push_str(&chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());
        html.push_str("</p>");

        // Summary metrics
        html.push_str("<h2>📊 Summary Metrics</h2>");
        html.push_str("<div>");

        let total_tests: usize = test_results.iter().map(|r| r.total_tests).sum();
        let total_passed: usize = test_results.iter().map(|r| r.passed_tests).sum();
        let total_failed: usize = test_results.iter().map(|r| r.failed_tests).sum();
        let overall_pass_rate = if total_tests > 0 {
            (total_passed as f64 / total_tests as f64 * 100.0)
        } else {
            0.0
        };

        html.push_str(&format!(
            r#"<div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Total Tests</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{}</div>
                <div class="metric-label">Tests Passed</div>
            </div>
            <div class="metric-card">
                <div class="metric-value">{:.1}%</div>
                <div class="metric-label">Pass Rate</div>
            </div>"#,
            total_tests, total_passed, overall_pass_rate
        ));

        html.push_str("</div>");

        // Test results table
        html.push_str("<h2>✅ Test Results by Component</h2>");
        html.push_str("<table>");
        html.push_str("<thead><tr><th>Component</th><th>Suite</th><th>Total</th><th>Passed</th><th>Failed</th><th>Pass Rate</th><th>Duration</th><th>Status</th></tr></thead>");
        html.push_str("<tbody>");

        for result in test_results {
            let pass_rate = result.pass_rate() * 100.0;
            let status_badge = if result.failed_tests == 0 {
                r#"<span class="badge badge-success">PASS</span>"#
            } else {
                r#"<span class="badge badge-danger">FAIL</span>"#
            };

            html.push_str(&format!(
                "<tr><td>{}</td><td>{}</td><td>{}</td><td class='pass'>{}</td><td class='fail'>{}</td><td>{:.1}%</td><td>{}ms</td><td>{}</td></tr>",
                result.component,
                result.test_suite,
                result.total_tests,
                result.passed_tests,
                result.failed_tests,
                pass_rate,
                result.duration.as_millis(),
                status_badge
            ));
        }

        html.push_str("</tbody></table>");

        // Quality metrics
        html.push_str("<h2>📈 Quality Metrics</h2>");
        if quality_metrics.is_empty() {
            html.push_str("<p>No quality metrics available.</p>");
        } else {
            html.push_str("<table>");
            html.push_str("<thead><tr><th>Metric</th><th>Value</th><th>Component</th></tr></thead>");
            html.push_str("<tbody>");

            for metric in quality_metrics {
                let component = metric.component.as_deref().unwrap_or("Project");
                let value_str = if let Some(unit) = &metric.metric_unit {
                    format!("{:.2} {}", metric.metric_value, unit)
                } else {
                    format!("{:.2}", metric.metric_value)
                };

                html.push_str(&format!(
                    "<tr><td>{}</td><td>{}</td><td>{}</td></tr>",
                    metric.metric_name, value_str, component
                ));
            }

            html.push_str("</tbody></table>");
        }

        html.push_str(r#"
    </div>
</body>
</html>"#);

        html
    }

    fn build_markdown(&self, test_results: &[TestRun], quality_metrics: &[QualityMetric]) -> String {
        let mut md = String::from("# FrankenBrowser Test Dashboard\n\n");

        md.push_str(&format!("**Generated:** {}\n\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S")));

        // Summary metrics
        md.push_str("## Summary Metrics\n\n");

        let total_tests: usize = test_results.iter().map(|r| r.total_tests).sum();
        let total_passed: usize = test_results.iter().map(|r| r.passed_tests).sum();
        let total_failed: usize = test_results.iter().map(|r| r.failed_tests).sum();
        let overall_pass_rate = if total_tests > 0 {
            (total_passed as f64 / total_tests as f64 * 100.0)
        } else {
            0.0
        };

        md.push_str(&format!("- **Total Tests:** {}\n", total_tests));
        md.push_str(&format!("- **Tests Passed:** {}\n", total_passed));
        md.push_str(&format!("- **Tests Failed:** {}\n", total_failed));
        md.push_str(&format!("- **Overall Pass Rate:** {:.1}%\n\n", overall_pass_rate));

        // Test results table
        md.push_str("## Test Results by Component\n\n");
        md.push_str("| Component | Suite | Total | Passed | Failed | Pass Rate | Duration | Status |\n");
        md.push_str("|-----------|-------|-------|--------|--------|-----------|----------|--------|\n");

        for result in test_results {
            let pass_rate = result.pass_rate() * 100.0;
            let status = if result.failed_tests == 0 { "✅ PASS" } else { "❌ FAIL" };

            md.push_str(&format!(
                "| {} | {} | {} | {} | {} | {:.1}% | {}ms | {} |\n",
                result.component,
                result.test_suite,
                result.total_tests,
                result.passed_tests,
                result.failed_tests,
                pass_rate,
                result.duration.as_millis(),
                status
            ));
        }

        md.push_str("\n");

        // Quality metrics
        md.push_str("## Quality Metrics\n\n");
        if quality_metrics.is_empty() {
            md.push_str("No quality metrics available.\n");
        } else {
            md.push_str("| Metric | Value | Component |\n");
            md.push_str("|--------|-------|----------|\n");

            for metric in quality_metrics {
                let component = metric.component.as_deref().unwrap_or("Project");
                let value_str = if let Some(unit) = &metric.metric_unit {
                    format!("{:.2} {}", metric.metric_value, unit)
                } else {
                    format!("{:.2}", metric.metric_value)
                };

                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    metric.metric_name, value_str, component
                ));
            }
        }

        md
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_markdown_generation() {
        let recorder = TestRecorder::new_in_memory().unwrap();
        let generator = DashboardGenerator::new(recorder);

        let test_results = vec![
            TestRun {
                component: "test1".to_string(),
                test_suite: "unit".to_string(),
                total_tests: 100,
                passed_tests: 100,
                failed_tests: 0,
                skipped_tests: 0,
                duration: Duration::from_secs(5),
                commit_hash: "abc123".to_string(),
                branch_name: "main".to_string(),
                metadata: None,
            }
        ];

        let markdown = generator.build_markdown(&test_results, &[]);
        assert!(markdown.contains("# FrankenBrowser Test Dashboard"));
        assert!(markdown.contains("test1"));
        assert!(markdown.contains("100%"));
    }
}
