-- Test tracking database schema for FrankenBrowser
-- Tracks test runs, performance benchmarks, and quality metrics over time

-- Table: test_runs
-- Records information about each test suite execution
CREATE TABLE IF NOT EXISTS test_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    component TEXT NOT NULL,                -- e.g., "webdriver", "browser_core", "all"
    test_suite TEXT NOT NULL,               -- e.g., "unit", "integration", "wpt"
    total_tests INTEGER NOT NULL,
    passed_tests INTEGER NOT NULL,
    failed_tests INTEGER NOT NULL,
    skipped_tests INTEGER NOT NULL,
    ignored_tests INTEGER NOT NULL DEFAULT 0,
    pass_rate REAL NOT NULL,                -- Calculated: passed / total
    commit_hash TEXT NOT NULL,
    branch TEXT,
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    duration_ms INTEGER,                    -- Test execution time in milliseconds
    runner TEXT,                            -- e.g., "github-actions", "local", "ci"
    rust_version TEXT,
    CONSTRAINT valid_pass_rate CHECK (pass_rate >= 0 AND pass_rate <= 1)
);

-- Table: performance_results
-- Records benchmark performance metrics
CREATE TABLE IF NOT EXISTS performance_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    benchmark TEXT NOT NULL,                -- Benchmark name
    mean_time_ns INTEGER NOT NULL,          -- Mean execution time in nanoseconds
    std_dev_ns INTEGER NOT NULL,            -- Standard deviation
    min_time_ns INTEGER NOT NULL,           -- Minimum time observed
    max_time_ns INTEGER NOT NULL,           -- Maximum time observed
    iterations INTEGER NOT NULL,            -- Number of iterations
    commit_hash TEXT NOT NULL,
    branch TEXT,
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    runner TEXT,
    CONSTRAINT valid_times CHECK (
        mean_time_ns >= 0 AND
        std_dev_ns >= 0 AND
        min_time_ns >= 0 AND
        max_time_ns >= mean_time_ns AND
        min_time_ns <= mean_time_ns
    )
);

-- Table: quality_metrics
-- Tracks code quality metrics over time
CREATE TABLE IF NOT EXISTS quality_metrics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    component TEXT NOT NULL,
    metric_name TEXT NOT NULL,              -- e.g., "coverage", "complexity", "lines_of_code"
    metric_value REAL NOT NULL,
    commit_hash TEXT NOT NULL,
    branch TEXT,
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Table: wpt_results
-- Specific tracking for Web Platform Tests
CREATE TABLE IF NOT EXISTS wpt_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    test_path TEXT NOT NULL,                -- Path to WPT test file
    status TEXT NOT NULL,                   -- PASS, FAIL, TIMEOUT, ERROR, SKIP
    message TEXT,                           -- Error message if failed
    duration_ms INTEGER,
    commit_hash TEXT NOT NULL,
    branch TEXT,
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    CONSTRAINT valid_status CHECK (status IN ('PASS', 'FAIL', 'TIMEOUT', 'ERROR', 'SKIP'))
);

-- Table: webdriver_endpoints
-- Track WebDriver endpoint implementation status
CREATE TABLE IF NOT EXISTS webdriver_endpoints (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    endpoint TEXT NOT NULL UNIQUE,          -- e.g., "POST /session/:id/url"
    implemented BOOLEAN NOT NULL DEFAULT 0,
    tested BOOLEAN NOT NULL DEFAULT 0,
    w3c_compliant BOOLEAN NOT NULL DEFAULT 0,
    notes TEXT,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Indexes for common queries
CREATE INDEX IF NOT EXISTS idx_test_runs_component ON test_runs(component);
CREATE INDEX IF NOT EXISTS idx_test_runs_suite ON test_runs(test_suite);
CREATE INDEX IF NOT EXISTS idx_test_runs_timestamp ON test_runs(run_timestamp);
CREATE INDEX IF NOT EXISTS idx_test_runs_commit ON test_runs(commit_hash);
CREATE INDEX IF NOT EXISTS idx_performance_benchmark ON performance_results(benchmark);
CREATE INDEX IF NOT EXISTS idx_performance_timestamp ON performance_results(run_timestamp);
CREATE INDEX IF NOT EXISTS idx_quality_component ON quality_metrics(component);
CREATE INDEX IF NOT EXISTS idx_quality_metric ON quality_metrics(metric_name);
CREATE INDEX IF NOT EXISTS idx_wpt_test ON wpt_results(test_path);
CREATE INDEX IF NOT EXISTS idx_wpt_status ON wpt_results(status);
CREATE INDEX IF NOT EXISTS idx_wpt_timestamp ON wpt_results(run_timestamp);

-- Views for common analytics queries

-- View: latest_test_results
-- Shows the most recent test run for each component/suite combination
CREATE VIEW IF NOT EXISTS latest_test_results AS
SELECT
    component,
    test_suite,
    total_tests,
    passed_tests,
    failed_tests,
    pass_rate,
    commit_hash,
    run_timestamp
FROM test_runs
WHERE id IN (
    SELECT MAX(id)
    FROM test_runs
    GROUP BY component, test_suite
)
ORDER BY component, test_suite;

-- View: test_trends
-- Shows pass rate trends over the last 30 days
CREATE VIEW IF NOT EXISTS test_trends AS
SELECT
    component,
    test_suite,
    DATE(run_timestamp) as date,
    AVG(pass_rate) as avg_pass_rate,
    COUNT(*) as runs_count
FROM test_runs
WHERE run_timestamp >= datetime('now', '-30 days')
GROUP BY component, test_suite, DATE(run_timestamp)
ORDER BY date DESC, component, test_suite;

-- View: performance_trends
-- Shows benchmark performance trends over the last 30 days
CREATE VIEW IF NOT EXISTS performance_trends AS
SELECT
    benchmark,
    DATE(run_timestamp) as date,
    AVG(mean_time_ns) as avg_mean_ns,
    MIN(min_time_ns) as best_time_ns,
    MAX(max_time_ns) as worst_time_ns,
    COUNT(*) as runs_count
FROM performance_results
WHERE run_timestamp >= datetime('now', '-30 days')
GROUP BY benchmark, DATE(run_timestamp)
ORDER BY date DESC, benchmark;

-- View: wpt_summary
-- Summary of WPT test results
CREATE VIEW IF NOT EXISTS wpt_summary AS
SELECT
    DATE(run_timestamp) as date,
    commit_hash,
    COUNT(*) as total_tests,
    SUM(CASE WHEN status = 'PASS' THEN 1 ELSE 0 END) as passed,
    SUM(CASE WHEN status = 'FAIL' THEN 1 ELSE 0 END) as failed,
    SUM(CASE WHEN status = 'TIMEOUT' THEN 1 ELSE 0 END) as timeout,
    SUM(CASE WHEN status = 'ERROR' THEN 1 ELSE 0 END) as error,
    SUM(CASE WHEN status = 'SKIP' THEN 1 ELSE 0 END) as skipped,
    CAST(SUM(CASE WHEN status = 'PASS' THEN 1 ELSE 0 END) AS REAL) / COUNT(*) as pass_rate
FROM wpt_results
GROUP BY DATE(run_timestamp), commit_hash
ORDER BY date DESC;
