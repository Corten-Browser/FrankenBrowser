# Test Tracking System

This directory contains the test tracking database and tools for FrankenBrowser.

## Overview

The test tracking system records:
- Test run results (unit, integration, WPT)
- Performance benchmarks
- Code quality metrics
- Trends over time

## Quick Start

```bash
# Initialize database
python3 test_tracker.py init

# Record a test run
python3 test_tracker.py record webdriver unit 17 17 0

# Generate dashboard
python3 test_tracker.py dashboard

# View trends
python3 test_tracker.py trends --days 30
```

## Database Schema

The database (`test_results.db`) contains:

- **test_runs**: Test execution results
- **performance_results**: Benchmark data
- **quality_metrics**: Code quality measurements
- **wpt_results**: Web Platform Test results
- **webdriver_endpoints**: WebDriver implementation status

## Usage Examples

### Record Test Results

```bash
# After running tests
cargo test --package webdriver --lib 2>&1 | tee /tmp/test_output.txt

# Parse results and record
# (assuming 17 tests passed)
python3 test_tracker.py record webdriver unit 17 17 0
```

### Generate Dashboard

```bash
# Output to terminal
python3 test_tracker.py dashboard

# Save to file
python3 test_tracker.py dashboard --output ../../docs/test-dashboard.md
```

### View Trends

```bash
# Last 30 days
python3 test_tracker.py trends

# Last 7 days
python3 test_tracker.py trends --days 7
```

## Integration with CI

In GitHub Actions:

```yaml
- name: Record test results
  if: always()
  run: |
    python3 tests/tracking/test_tracker.py record \
      "all components" \
      "ci" \
      $TOTAL_TESTS \
      $PASSED_TESTS \
      $FAILED_TESTS \
      --runner github-actions
```

## Database Queries

The schema includes useful views:

```sql
-- Latest results
SELECT * FROM latest_test_results;

-- Trends
SELECT * FROM test_trends;

-- Performance
SELECT * FROM performance_trends;

-- WPT summary
SELECT * FROM wpt_summary;
```

## Files

- `schema.sql`: Database schema definition
- `test_tracker.py`: Main tracking tool
- `test_results.db`: SQLite database (gitignored)
- `README.md`: This file
