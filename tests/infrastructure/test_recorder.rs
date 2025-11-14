use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::Duration;

/// Test run record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestRun {
    pub component: String,
    pub test_suite: String,
    pub total_tests: usize,
    pub passed_tests: usize,
    pub failed_tests: usize,
    pub skipped_tests: usize,
    pub duration: Duration,
    pub commit_hash: String,
    pub branch_name: String,
    pub metadata: Option<String>,
}

impl TestRun {
    pub fn pass_rate(&self) -> f64 {
        if self.total_tests == 0 {
            0.0
        } else {
            self.passed_tests as f64 / self.total_tests as f64
        }
    }
}

/// Performance benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    pub benchmark_name: String,
    pub component: String,
    pub mean_time_ns: i64,
    pub std_dev_ns: i64,
    pub min_time_ns: i64,
    pub max_time_ns: i64,
    pub throughput: Option<f64>,
    pub commit_hash: String,
    pub branch_name: String,
    pub metadata: Option<String>,
}

/// Test failure record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFailure {
    pub test_name: String,
    pub failure_message: String,
    pub stack_trace: Option<String>,
    pub test_file: Option<String>,
    pub test_line: Option<usize>,
}

/// Quality metric record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityMetric {
    pub metric_name: String,
    pub metric_value: f64,
    pub metric_unit: Option<String>,
    pub component: Option<String>,
    pub commit_hash: String,
    pub branch_name: String,
    pub metadata: Option<String>,
}

/// Test recorder for persisting test results
pub struct TestRecorder {
    conn: Connection,
}

impl TestRecorder {
    /// Create a new test recorder with the given database path
    pub fn new<P: AsRef<Path>>(db_path: P) -> Result<Self> {
        let conn = Connection::open(db_path)?;

        // Initialize schema
        let schema = include_str!("schema.sql");
        conn.execute_batch(schema)?;

        Ok(Self { conn })
    }

    /// Create an in-memory test recorder for testing
    pub fn new_in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;

        // Initialize schema
        let schema = include_str!("schema.sql");
        conn.execute_batch(schema)?;

        Ok(Self { conn })
    }

    /// Record a test run
    pub fn record_test_run(&mut self, run: &TestRun) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO test_runs (
                component, test_suite, total_tests, passed_tests, failed_tests,
                skipped_tests, pass_rate, duration_ms, commit_hash, branch_name, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
            params![
                run.component,
                run.test_suite,
                run.total_tests,
                run.passed_tests,
                run.failed_tests,
                run.skipped_tests,
                run.pass_rate(),
                run.duration.as_millis() as i64,
                run.commit_hash,
                run.branch_name,
                run.metadata,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Record test failures for a test run
    pub fn record_test_failures(&mut self, test_run_id: i64, failures: &[TestFailure]) -> Result<()> {
        for failure in failures {
            self.conn.execute(
                "INSERT INTO test_failures (
                    test_run_id, test_name, failure_message, stack_trace, test_file, test_line
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    test_run_id,
                    failure.test_name,
                    failure.failure_message,
                    failure.stack_trace,
                    failure.test_file,
                    failure.test_line.map(|l| l as i64),
                ],
            )?;
        }

        Ok(())
    }

    /// Record a benchmark result
    pub fn record_benchmark(&mut self, benchmark: &BenchmarkResult) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO performance_results (
                benchmark_name, component, mean_time_ns, std_dev_ns, min_time_ns,
                max_time_ns, throughput, commit_hash, branch_name, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![
                benchmark.benchmark_name,
                benchmark.component,
                benchmark.mean_time_ns,
                benchmark.std_dev_ns,
                benchmark.min_time_ns,
                benchmark.max_time_ns,
                benchmark.throughput,
                benchmark.commit_hash,
                benchmark.branch_name,
                benchmark.metadata,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Record a quality metric
    pub fn record_quality_metric(&mut self, metric: &QualityMetric) -> Result<i64> {
        self.conn.execute(
            "INSERT INTO quality_metrics (
                metric_name, metric_value, metric_unit, component, commit_hash, branch_name, metadata
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                metric.metric_name,
                metric.metric_value,
                metric.metric_unit,
                metric.component,
                metric.commit_hash,
                metric.branch_name,
                metric.metadata,
            ],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    /// Get latest test results for all components
    pub fn get_latest_results(&self) -> Result<Vec<TestRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT component, test_suite, total_tests, passed_tests, failed_tests,
                    skipped_tests, duration_ms, commit_hash, branch_name, metadata
             FROM latest_test_results"
        )?;

        let results = stmt.query_map([], |row| {
            Ok(TestRun {
                component: row.get(0)?,
                test_suite: row.get(1)?,
                total_tests: row.get(2)?,
                passed_tests: row.get(3)?,
                failed_tests: row.get(4)?,
                skipped_tests: row.get(5)?,
                duration: Duration::from_millis(row.get::<_, i64>(6)? as u64),
                commit_hash: row.get(7)?,
                branch_name: row.get(8)?,
                metadata: row.get(9)?,
            })
        })?;

        results.collect()
    }

    /// Get performance trends for a benchmark
    pub fn get_performance_trend(&self, benchmark_name: &str) -> Result<Vec<BenchmarkResult>> {
        let mut stmt = self.conn.prepare(
            "SELECT benchmark_name, component, mean_time_ns, std_dev_ns, min_time_ns,
                    max_time_ns, throughput, commit_hash, branch_name, metadata
             FROM performance_results
             WHERE benchmark_name = ?1
             ORDER BY run_timestamp DESC
             LIMIT 100"
        )?;

        let results = stmt.query_map(params![benchmark_name], |row| {
            Ok(BenchmarkResult {
                benchmark_name: row.get(0)?,
                component: row.get(1)?,
                mean_time_ns: row.get(2)?,
                std_dev_ns: row.get(3)?,
                min_time_ns: row.get(4)?,
                max_time_ns: row.get(5)?,
                throughput: row.get(6)?,
                commit_hash: row.get(7)?,
                branch_name: row.get(8)?,
                metadata: row.get(9)?,
            })
        })?;

        results.collect()
    }

    /// Get quality metrics summary
    pub fn get_quality_summary(&self) -> Result<Vec<QualityMetric>> {
        let mut stmt = self.conn.prepare(
            "SELECT metric_name, metric_value, metric_unit, component, commit_hash, branch_name
             FROM quality_summary"
        )?;

        let results = stmt.query_map([], |row| {
            Ok(QualityMetric {
                metric_name: row.get(0)?,
                metric_value: row.get(1)?,
                metric_unit: row.get(2)?,
                component: row.get(3)?,
                commit_hash: row.get(4)?,
                branch_name: row.get(5)?,
                metadata: None,
            })
        })?;

        results.collect()
    }

    /// Get current Git commit hash
    pub fn get_current_commit() -> Result<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .output()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("unknown".to_string())
        }
    }

    /// Get current Git branch name
    pub fn get_current_branch() -> Result<String> {
        use std::process::Command;

        let output = Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Ok("unknown".to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_recorder_creation() {
        let recorder = TestRecorder::new_in_memory();
        assert!(recorder.is_ok());
    }

    #[test]
    fn test_record_test_run() {
        let mut recorder = TestRecorder::new_in_memory().unwrap();

        let run = TestRun {
            component: "test_component".to_string(),
            test_suite: "unit".to_string(),
            total_tests: 100,
            passed_tests: 95,
            failed_tests: 5,
            skipped_tests: 0,
            duration: Duration::from_secs(10),
            commit_hash: "abc123".to_string(),
            branch_name: "main".to_string(),
            metadata: None,
        };

        let result = recorder.record_test_run(&run);
        assert!(result.is_ok());
    }

    #[test]
    fn test_record_benchmark() {
        let mut recorder = TestRecorder::new_in_memory().unwrap();

        let benchmark = BenchmarkResult {
            benchmark_name: "test_bench".to_string(),
            component: "test_component".to_string(),
            mean_time_ns: 1000,
            std_dev_ns: 100,
            min_time_ns: 900,
            max_time_ns: 1100,
            throughput: Some(1000.0),
            commit_hash: "abc123".to_string(),
            branch_name: "main".to_string(),
            metadata: None,
        };

        let result = recorder.record_benchmark(&benchmark);
        assert!(result.is_ok());
    }

    #[test]
    fn test_get_latest_results() {
        let mut recorder = TestRecorder::new_in_memory().unwrap();

        // Record a test run
        let run = TestRun {
            component: "test_component".to_string(),
            test_suite: "unit".to_string(),
            total_tests: 50,
            passed_tests: 50,
            failed_tests: 0,
            skipped_tests: 0,
            duration: Duration::from_secs(5),
            commit_hash: "xyz789".to_string(),
            branch_name: "main".to_string(),
            metadata: None,
        };

        recorder.record_test_run(&run).unwrap();

        // Retrieve latest results
        let results = recorder.get_latest_results().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].component, "test_component");
        assert_eq!(results[0].passed_tests, 50);
    }
}
