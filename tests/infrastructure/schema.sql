-- Test Infrastructure Database Schema
-- Tracks test runs, results, and performance metrics over time

-- Table: test_runs
-- Records each test suite execution
CREATE TABLE IF NOT EXISTS test_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    component TEXT NOT NULL,                -- Component name (e.g., "message_bus", "browser_core")
    test_suite TEXT NOT NULL,               -- Test suite type (e.g., "unit", "integration", "wpt")
    total_tests INTEGER NOT NULL,           -- Total number of tests in suite
    passed_tests INTEGER NOT NULL,          -- Number of tests that passed
    failed_tests INTEGER NOT NULL,          -- Number of tests that failed
    skipped_tests INTEGER NOT NULL,         -- Number of tests skipped
    pass_rate REAL NOT NULL,                -- Pass rate (0.0 to 1.0)
    duration_ms INTEGER NOT NULL,           -- Test execution time in milliseconds
    commit_hash TEXT NOT NULL,              -- Git commit hash
    branch_name TEXT NOT NULL,              -- Git branch name
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT                           -- JSON metadata for additional info
);

-- Table: performance_results
-- Records benchmark results for performance tracking
CREATE TABLE IF NOT EXISTS performance_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    benchmark_name TEXT NOT NULL,           -- Name of the benchmark
    component TEXT NOT NULL,                -- Component being benchmarked
    mean_time_ns INTEGER NOT NULL,          -- Mean execution time in nanoseconds
    std_dev_ns INTEGER NOT NULL,            -- Standard deviation in nanoseconds
    min_time_ns INTEGER NOT NULL,           -- Minimum time observed
    max_time_ns INTEGER NOT NULL,           -- Maximum time observed
    throughput REAL,                        -- Operations per second (if applicable)
    commit_hash TEXT NOT NULL,              -- Git commit hash
    branch_name TEXT NOT NULL,              -- Git branch name
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT                           -- JSON metadata for additional info
);

-- Table: test_failures
-- Detailed information about test failures
CREATE TABLE IF NOT EXISTS test_failures (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_run_id INTEGER NOT NULL,           -- Foreign key to test_runs
    test_name TEXT NOT NULL,                -- Name of the failing test
    failure_message TEXT NOT NULL,          -- Error/failure message
    stack_trace TEXT,                       -- Stack trace if available
    test_file TEXT,                         -- Source file of the test
    test_line INTEGER,                      -- Line number of the test
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (test_run_id) REFERENCES test_runs(id)
);

-- Table: quality_metrics
-- Overall project quality metrics
CREATE TABLE IF NOT EXISTS quality_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    metric_name TEXT NOT NULL,              -- Metric name (e.g., "coverage", "clippy_warnings")
    metric_value REAL NOT NULL,             -- Numeric value of the metric
    metric_unit TEXT,                       -- Unit (e.g., "%", "count")
    component TEXT,                         -- Component (null for project-wide)
    commit_hash TEXT NOT NULL,              -- Git commit hash
    branch_name TEXT NOT NULL,              -- Git branch name
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    metadata TEXT                           -- JSON metadata
);

-- Table: wpt_results
-- Web Platform Test results
CREATE TABLE IF NOT EXISTS wpt_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_path TEXT NOT NULL,                -- WPT test path
    test_name TEXT NOT NULL,                -- Test name
    status TEXT NOT NULL,                   -- "PASS", "FAIL", "TIMEOUT", "SKIP"
    expected_status TEXT,                   -- Expected status
    message TEXT,                           -- Test message/output
    duration_ms INTEGER,                    -- Test duration
    commit_hash TEXT NOT NULL,              -- Git commit hash
    branch_name TEXT NOT NULL,              -- Git branch name
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_test_runs_component ON test_runs(component);
CREATE INDEX IF NOT EXISTS idx_test_runs_timestamp ON test_runs(run_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_test_runs_commit ON test_runs(commit_hash);
CREATE INDEX IF NOT EXISTS idx_performance_component ON performance_results(component);
CREATE INDEX IF NOT EXISTS idx_performance_benchmark ON performance_results(benchmark_name);
CREATE INDEX IF NOT EXISTS idx_performance_timestamp ON performance_results(run_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_quality_metrics_name ON quality_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_quality_metrics_timestamp ON quality_metrics(run_timestamp DESC);
CREATE INDEX IF NOT EXISTS idx_wpt_results_status ON wpt_results(status);
CREATE INDEX IF NOT EXISTS idx_wpt_results_timestamp ON wpt_results(run_timestamp DESC);

-- Views for common queries

-- View: latest_test_results
-- Shows the most recent test run for each component
CREATE VIEW IF NOT EXISTS latest_test_results AS
SELECT
    component,
    test_suite,
    total_tests,
    passed_tests,
    failed_tests,
    pass_rate,
    duration_ms,
    commit_hash,
    run_timestamp
FROM test_runs
WHERE run_timestamp IN (
    SELECT MAX(run_timestamp)
    FROM test_runs
    GROUP BY component, test_suite
)
ORDER BY component, test_suite;

-- View: performance_trends
-- Shows performance trends over last 30 days
CREATE VIEW IF NOT EXISTS performance_trends AS
SELECT
    benchmark_name,
    component,
    AVG(mean_time_ns) as avg_time_ns,
    MIN(mean_time_ns) as best_time_ns,
    MAX(mean_time_ns) as worst_time_ns,
    COUNT(*) as run_count,
    MAX(run_timestamp) as latest_run
FROM performance_results
WHERE run_timestamp >= datetime('now', '-30 days')
GROUP BY benchmark_name, component
ORDER BY component, benchmark_name;

-- View: quality_summary
-- Current project quality summary
CREATE VIEW IF NOT EXISTS quality_summary AS
SELECT
    metric_name,
    metric_value,
    metric_unit,
    component,
    commit_hash,
    run_timestamp
FROM quality_metrics
WHERE run_timestamp IN (
    SELECT MAX(run_timestamp)
    FROM quality_metrics
    GROUP BY metric_name, COALESCE(component, '')
)
ORDER BY metric_name, component;

-- View: wpt_pass_rate
-- WPT pass rate summary
CREATE VIEW IF NOT EXISTS wpt_pass_rate AS
SELECT
    COUNT(*) as total_tests,
    SUM(CASE WHEN status = 'PASS' THEN 1 ELSE 0 END) as passed,
    SUM(CASE WHEN status = 'FAIL' THEN 1 ELSE 0 END) as failed,
    SUM(CASE WHEN status = 'TIMEOUT' THEN 1 ELSE 0 END) as timeout,
    SUM(CASE WHEN status = 'SKIP' THEN 1 ELSE 0 END) as skipped,
    ROUND(100.0 * SUM(CASE WHEN status = 'PASS' THEN 1 ELSE 0 END) / COUNT(*), 2) as pass_rate,
    MAX(run_timestamp) as latest_run
FROM wpt_results
WHERE run_timestamp IN (
    SELECT MAX(run_timestamp)
    FROM wpt_results
);
