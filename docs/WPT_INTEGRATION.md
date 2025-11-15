# Web Platform Tests (WPT) Integration Guide

**Status**: Planning/Documentation Phase
**Target**: 40% pass rate per specification
**Current**: 0% (not yet running)

## Overview

The Web Platform Tests (WPT) are a cross-browser test suite for web platform features. This document outlines the integration plan for running WPT against FrankenBrowser.

## Architecture

```
┌─────────────────────────────────────────────┐
│         Web Platform Tests Repository       │
│         https://github.com/web-platform-    │
│         tests/wpt                           │
└─────────────────┬───────────────────────────┘
                  │
                  ├─ test manifests
                  ├─ test files (.html, .js)
                  └─ test harness (testharness.js)
                  │
                  ▼
┌─────────────────────────────────────────────┐
│         WPT Test Runner (Python)             │
│         ./wpt run frankenbrowser            │
└─────────────────┬───────────────────────────┘
                  │
                  ├─ HTTP
                  ▼
┌─────────────────────────────────────────────┐
│    WebDriver Server (Port 4444)              │
│    components/webdriver/                     │
└─────────────────┬───────────────────────────┘
                  │
                  ├─ WebDriver Protocol
                  ▼
┌─────────────────────────────────────────────┐
│    FrankenBrowser (GUI Mode)                 │
│    cargo run --features gui                  │
└──────────────────────────────────────────────┘
```

## Prerequisites

### 1. WebDriver Protocol Implementation

**Status**: ✅ Core structure complete (simplified)

The WebDriver component provides:
- Session management
- Navigation commands
- Element location (placeholder)
- Script execution (placeholder)
- Screenshot capability (placeholder)

**Full HTTP server implementation**: Pending (requires axum integration)

### 2. WPT Repository

Clone the Web Platform Tests repository:

```bash
git clone https://github.com/web-platform-tests/wpt.git
cd wpt
```

### 3. Python Dependencies

Install WPT test runner:

```bash
pip install requests pytest
```

### 4. Browser Product Configuration

Create `frankenbrowser.ini` in wpt repository:

```ini
[products]
frankenbrowser = {
    "binary": "/path/to/frankenbrowser",
    "webdriver_binary": "/path/to/webdriver-server",
    "browser_channel": "dev"
}

[frankenbrowser]
binary = /home/user/FrankenBrowser/target/release/frankenbrowser
webdriver_binary = /home/user/FrankenBrowser/target/release/webdriver-server
prefs = {}
```

## Running WPT Tests

### Basic Usage

```bash
# Run all tests
./wpt run frankenbrowser

# Run specific test directory
./wpt run frankenbrowser html/dom/

# Run specific test file
./wpt run frankenbrowser html/semantics/forms/the-input-element/input-type-url.html

# Run with specific options
./wpt run --log-mach=- --log-mach-level=info frankenbrowser
```

### Test Categories

WPT tests are organized by specification area:

- `css/` - CSS specifications
- `dom/` - DOM APIs
- `html/` - HTML elements and APIs
- `fetch/` - Fetch API
- `xhr/` - XMLHttpRequest
- `websockets/` - WebSocket API
- `webdriver/` - WebDriver protocol tests

**Initial Focus**: Start with basic HTML and DOM tests

### Expected Pass Rates

| Category | Expected Initial | Target |
|----------|------------------|--------|
| HTML Parsing | 60-70% | 80%+ |
| DOM APIs | 40-50% | 70%+ |
| CSS Selectors | 50-60% | 75%+ |
| JavaScript | 30-40% | 60%+ |
| Fetch/XHR | 20-30% | 50%+ |
| **Overall** | **35-45%** | **40%+** |

## Integration Steps

### Phase 1: WebDriver Server Setup (Estimated: 4-6 hours)

**TODO**: Complete HTTP server implementation

```bash
# Build webdriver server binary
cargo build --release --bin webdriver-server --features webdriver-server

# Start server
./target/release/webdriver-server --port 4444
```

**Files to create**:
- `components/webdriver/src/bin/webdriver-server.rs` - Server binary

**Implementation**:
```rust
// components/webdriver/src/bin/webdriver-server.rs
use webdriver::server::start_server;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let port = std::env::var("WEBDRIVER_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(4444);

    println!("Starting WebDriver server on port {}", port);

    if let Err(e) = start_server(port).await {
        eprintln!("WebDriver server error: {}", e);
        std::process::exit(1);
    }
}
```

### Phase 2: WPT Product Adapter (Estimated: 2-3 hours)

**TODO**: Create WPT product adapter for FrankenBrowser

**Files to create**:
- `wpt_integration/products.py` - Product adapter
- `wpt_integration/metadata/` - Test expectations

**Implementation**:
```python
# wpt_integration/products.py
def setup_frankenbrowser(logger, binary, **kwargs):
    """Set up FrankenBrowser for WPT testing."""
    return {
        "browser": binary,
        "webdriver_url": "http://localhost:4444/",
        "capabilities": {
            "browserName": "frankenbrowser",
            "browserVersion": "0.1.0"
        }
    }
```

### Phase 3: Test Execution Infrastructure (Estimated: 2-3 hours)

**TODO**: Create test harness scripts

```bash
#!/bin/bash
# wpt_integration/run_wpt.sh

# Start WebDriver server
./target/release/webdriver-server --port 4444 &
WEBDRIVER_PID=$!

# Wait for server to start
sleep 2

# Run WPT tests
cd wpt
./wpt run frankenbrowser --log-wptreport ../wpt_results.json

# Stop WebDriver server
kill $WEBDRIVER_PID

# Generate report
python ../wpt_integration/generate_report.py ../wpt_results.json
```

### Phase 4: Results Analysis (Estimated: 1-2 hours)

**TODO**: Analyze test results and identify gaps

```python
# wpt_integration/generate_report.py
import json
import sys

def analyze_results(results_file):
    with open(results_file) as f:
        results = json.load(f)

    total = 0
    passed = 0
    failed = 0

    for test in results['results']:
        total += 1
        if test['status'] == 'PASS':
            passed += 1
        else:
            failed += 1

    pass_rate = (passed / total * 100) if total > 0 else 0

    print(f"WPT Results:")
    print(f"  Total: {total}")
    print(f"  Passed: {passed}")
    print(f"  Failed: {failed}")
    print(f"  Pass Rate: {pass_rate:.1f}%")

    return pass_rate

if __name__ == "__main__":
    pass_rate = analyze_results(sys.argv[1])
    sys.exit(0 if pass_rate >= 40 else 1)
```

## Known Limitations

### Current Blockers

1. **Xvfb Compatibility**: Window handle issue prevents GUI testing
   - **Workaround**: Test on real X11 display
   - **Impact**: Cannot run WPT in CI/CD yet

2. **WebDriver HTTP Server**: Not fully implemented
   - **Status**: Core structure and types complete
   - **Remaining**: HTTP endpoint implementation with axum
   - **Estimated**: 4-6 hours

3. **Element Location**: Placeholder implementation
   - **Impact**: Tests requiring DOM queries will fail
   - **Estimated**: 3-4 hours for basic implementation

4. **Script Execution**: Placeholder implementation
   - **Impact**: JavaScript-dependent tests will fail
   - **Estimated**: 2-3 hours for WebView integration

### Partial Implementations

- Screenshot API: Placeholder PNG (not real capture)
- Navigation: API ready, display blocked by GUI issue
- Session management: ✅ Complete and tested

## Achieving 40% Pass Rate

### Low-Hanging Fruit (Estimated: 8-12 hours)

1. **HTML Parsing Tests** (Expected: 60-70% pass)
   - WebKit handles this
   - Minimal FrankenBrowser code needed
   - High impact

2. **Basic DOM Queries** (Expected: 50-60% pass)
   - Implement element location
   - Connect to WebView DOM access
   - Medium impact

3. **CSS Selector Tests** (Expected: 50-60% pass)
   - WebKit handles this
   - Requires element location
   - Medium impact

### Medium Effort (Estimated: 12-16 hours)

4. **JavaScript Execution** (Expected: 30-40% pass)
   - Connect script execution to WebView
   - Return actual results
   - High impact

5. **Navigation Tests** (Expected: 60-70% pass)
   - Fix GUI issue or accept desktop-only testing
   - Verify page load completion
   - High impact

### Optional (For >50% pass rate)

6. **Fetch/XHR Tests** (Expected: 20-30% pass)
   - Network stack is ready
   - Expose via JavaScript
   - Lower priority

## Testing Strategy

### Incremental Approach

**Week 1**: Foundation
1. Complete WebDriver HTTP server (6 hours)
2. Set up WPT repository and tooling (2 hours)
3. Run initial test suite (baseline) (1 hour)

**Week 2**: Core Features
4. Implement element location (4 hours)
5. Implement script execution (3 hours)
6. Run tests, analyze failures (2 hours)

**Week 3**: Optimization
7. Fix top 10 failing test patterns (8 hours)
8. Re-run tests, measure progress (1 hour)
9. Iterate until 40% pass rate (variable)

### Continuous Monitoring

```bash
# Daily test run
./wpt_integration/run_wpt.sh

# Track progress
git log --grep="WPT" --oneline
```

## Success Criteria

✅ **Minimum Viable**:
- WebDriver server responds to requests
- At least one test passes end-to-end
- Results can be collected and analyzed

✅ **Specification Requirement**:
- 40% overall pass rate
- Consistent results (reproducible)
- Results logged and tracked

✅ **Stretch Goals**:
- 50%+ pass rate
- CI/CD integration
- Automated regression detection

## Resources

- [WPT Documentation](https://web-platform-tests.org/)
- [WebDriver Specification](https://w3c.github.io/webdriver/)
- [WPT GitHub Repository](https://github.com/web-platform-tests/wpt)
- [WPT Infrastructure](https://github.com/web-platform-tests/wpt-infrastructure)

## Next Steps

1. **Complete WebDriver HTTP server** (4-6 hours)
   - Fix axum integration
   - Implement all endpoints
   - Test with sample requests

2. **Create WPT integration scripts** (2-3 hours)
   - Product adapter
   - Test runner script
   - Results analyzer

3. **Run baseline tests** (1 hour)
   - Execute WPT suite
   - Collect results
   - Identify quick wins

4. **Implement element location** (3-4 hours)
   - Connect to WebView DOM
   - Support CSS selectors
   - Test with WPT

5. **Achieve 40% pass rate** (variable)
   - Fix failing patterns
   - Iterate and test
   - Document results

---

**Created**: 2025-11-14
**Status**: Planning/Documentation
**Estimated Total Effort**: 14-22 hours
**Priority**: Medium (after GUI issue resolution)
