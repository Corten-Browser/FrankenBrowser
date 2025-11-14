# FrankenBrowser Test Execution Guide

## Overview

This guide covers all test types available in the FrankenBrowser project and how to run them.

## Test Types

### 1. Unit Tests

**What they test:** Individual component functionality in isolation

**Location:** `components/*/tests/`

**Run all unit tests:**
```bash
cargo test --lib --all
```

**Run specific component tests:**
```bash
cargo test --lib -p message_bus
cargo test --lib -p browser_core
cargo test --lib -p adblock_engine
```

**Expected results:**
- 223 total tests
- 100% pass rate
- Coverage: >80% per component

### 2. Integration Tests

**What they test:** Component interaction and integration

**Location:** `integration_tests/tests/`

**Run all integration tests:**
```bash
cargo test --test '*'
```

**Run specific integration test suites:**
```bash
cargo test --test test_message_bus_integration
cargo test --test test_network_integration
cargo test --test test_complete_workflows
```

**Expected results:**
- 42 total tests (41 executed, 1 ignored)
- 100% pass rate on executed tests

### 3. Validation Tests

**What they test:** Component lifecycle, configuration, security

**Location:** `tests/validation/`

**Run all validation tests:**
```bash
cargo test --test validation::component_lifecycle
cargo test --test validation::configuration
cargo test --test validation::security
```

**Run all validation tests together:**
```bash
cd tests/validation && cargo test
```

**Expected results:**
- Component lifecycle: 10+ tests passing
- Configuration: 15+ tests passing
- Security: 15+ tests passing
- All tests should pass (100%)

### 4. Performance Benchmarks

**What they test:** Component performance and throughput

**Location:** `benches/performance.rs`

**Run all benchmarks:**
```bash
cargo bench
```

**Run specific benchmarks:**
```bash
cargo bench --bench performance -- message_bus
cargo bench --bench performance -- adblock
```

**View results:**
```bash
open target/criterion/report/index.html
```

**Benchmark categories:**
- Message bus operations
- Configuration management
- Ad block filter matching
- Message serialization
- Concurrent operations

### 5. WPT (Web Platform Tests) - Infrastructure Only

**Status:** 🚧 Structure complete, requires WebDriver support

**Location:** `tests/wpt/`

**Test infrastructure:**
```bash
cd tests/wpt
cargo test
```

**What's missing:** Headless mode + WebDriver protocol

See `tests/wpt/README.md` for details.

### 6. ACID Tests - Infrastructure Only

**Status:** 🚧 Structure complete, requires screenshot API

**Location:** `tests/acid/`

**Test infrastructure:**
```bash
cd tests/acid
cargo test
```

**What's missing:** Screenshot capture + image comparison

See `tests/acid/README.md` for details.

## Quick Commands

### Run Everything (Implemented)

```bash
# All unit tests
cargo test --lib --all

# All integration tests
cargo test --test '*'

# All validation tests
cd tests/validation && cargo test

# All benchmarks
cargo bench

# Code quality checks
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings
```

### Generate Reports

**Test coverage:**
```bash
cargo install cargo-tarpaulin
cargo tarpaulin --out Html --all
open tarpaulin-report.html
```

**Benchmark reports:**
```bash
cargo bench
open target/criterion/report/index.html
```

## Continuous Integration

### GitHub Actions

The project includes automated CI/CD:

**Test Suite Workflow** (`.github/workflows/test-suite.yml`):
- Unit tests
- Integration tests
- Validation tests
- Code quality checks
- Build verification
- Test coverage

**Benchmark Workflow** (`.github/workflows/benchmark-regression.yml`):
- Performance benchmarks
- Regression detection
- Trend analysis

**Triggered on:**
- Push to `main` or `claude/**` branches
- Pull requests to `main`

**View results:**
- GitHub Actions tab in repository
- PR checks

## Test Database

### Recording Test Results

```rust
use test_recorder::{TestRecorder, TestRun};

let mut recorder = TestRecorder::new("tests/test_results.db")?;

let run = TestRun {
    component: "browser_core".to_string(),
    test_suite: "unit".to_string(),
    total_tests: 45,
    passed_tests: 45,
    failed_tests: 0,
    skipped_tests: 0,
    duration: Duration::from_secs(2),
    commit_hash: "abc123".to_string(),
    branch_name: "main".to_string(),
    metadata: None,
};

recorder.record_test_run(&run)?;
```

### Viewing Test History

```bash
# Query test database
sqlite3 tests/test_results.db "SELECT * FROM latest_test_results;"

# View quality summary
sqlite3 tests/test_results.db "SELECT * FROM quality_summary;"
```

## Test Dashboard

### Generate HTML Dashboard

```bash
# Generate dashboard from test results
cargo run --bin generate_dashboard

# View dashboard
open docs/test_dashboard.html
```

### Dashboard Contents

- Overall test metrics
- Component-by-component results
- Pass/fail trends
- Performance benchmarks
- Quality metrics

## Troubleshooting

### Tests Fail to Compile

```bash
# Clean and rebuild
cargo clean
cargo build --all

# Check for missing dependencies
cargo check --all
```

### Tests Hang

- Check for infinite loops in test code
- Verify timeout configuration
- Run with `--test-threads=1` to isolate

```bash
cargo test -- --test-threads=1
```

### Benchmark Fails

```bash
# Ensure release mode
cargo bench --release

# Check system resources
# Benchmarks may be unstable on busy systems
```

### Coverage Tool Fails

```bash
# Reinstall tarpaulin
cargo install --force cargo-tarpaulin

# Run with verbose output
cargo tarpaulin --out Html --verbose
```

## Best Practices

### Before Committing

```bash
# Run full test suite
cargo test --all

# Check code quality
cargo fmt --all -- --check
cargo clippy --all --all-targets -- -D warnings

# Run benchmarks (optional but recommended)
cargo bench
```

### Writing New Tests

1. **Unit tests:** Add to `components/*/tests/`
2. **Integration tests:** Add to `integration_tests/tests/`
3. **Validation tests:** Add to `tests/validation/`
4. **Benchmarks:** Add to `benches/performance.rs`

### Test Organization

- Keep unit tests close to code they test
- Use descriptive test names
- Follow TDD: Red-Green-Refactor
- Aim for >80% coverage
- Test edge cases and error paths

## Performance Targets

### Unit Tests

- Total execution time: <30 seconds
- Individual test: <100ms
- No test should take >1 second

### Integration Tests

- Total execution time: <60 seconds
- Individual test: <1 second

### Benchmarks

- Complete run: <10 minutes
- Individual benchmark: <30 seconds

## Next Steps

After running current tests:

1. Review test coverage reports
2. Add tests for uncovered code paths
3. Implement missing test capabilities (WPT, ACID)
4. Set up continuous monitoring

See `docs/testing/ROADMAP.md` for future test development.
