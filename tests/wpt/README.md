# Web Platform Tests (WPT) for FrankenBrowser

## Overview

This directory contains the infrastructure for running Web Platform Tests against FrankenBrowser.

**Current Status:** 🚧 Infrastructure complete, execution requires additional browser capabilities

## What's Here

- `config.toml` - WPT configuration (test paths, subsets, pass rate targets)
- `harness.rs` - WPT test harness implementation
- `README.md` - This file

## What's Needed to Run WPT

### Prerequisites

1. **Clone WPT Repository**
   ```bash
   cd tests
   git clone --depth=1 https://github.com/web-platform-tests/wpt.git
   ```

2. **Build Browser**
   ```bash
   cargo build --release
   ```

### Required Browser Capabilities (Not Yet Implemented)

To run WPT tests, FrankenBrowser needs:

#### 1. Headless Mode Support
- Modify `browser_shell` to run without GUI
- Use virtual framebuffer (Xvfb) in CI
- Add `--headless` flag to CLI

#### 2. WebDriver Protocol
- Implement basic WebDriver HTTP endpoints
- Support commands:
  - Navigate to URL
  - Execute JavaScript
  - Get page source
  - Take screenshot
  - Get/set cookies

#### 3. Automation API
- DOM inspection
- Element queries
- Event triggering
- Network interception

## Phase 1 Target

**Expected Pass Rate:** 40% (using existing WebView components)

**Test Subsets:**
- `html/browsers/browsing-the-web/navigating-across-documents`
- `html/browsers/history`
- `fetch/api/basic`
- `dom/events`
- `html/semantics`

## Usage (Once Implemented)

### Run Full WPT Suite
```bash
cargo test --test wpt_runner
```

### Run Specific Subset
```bash
cargo test --test wpt_runner -- --subset html/browsers/history
```

### Generate HTML Report
```bash
cargo test --test wpt_runner -- --report-html
```

## Implementation Roadmap

### Phase 1: Infrastructure ✅ (Complete)
- [x] Configuration system
- [x] Harness structure
- [x] Result tracking
- [x] Placeholder tests

### Phase 2: Headless Mode (TODO)
- [ ] Add headless flag to browser_shell
- [ ] Virtual display support (Xvfb)
- [ ] Headless WebView configuration

### Phase 3: WebDriver Protocol (TODO)
- [ ] HTTP server for WebDriver
- [ ] Navigate command
- [ ] Execute script command
- [ ] Element location commands
- [ ] Screenshot command

### Phase 4: Full Execution (TODO)
- [ ] Run complete WPT suite
- [ ] Achieve 40% pass rate target
- [ ] HTML reporting
- [ ] CI integration

## Current Limitations

⚠️ **Cannot Execute Tests Yet**

The test infrastructure is complete but cannot run actual WPT tests because:

1. No headless mode - browser requires display
2. No WebDriver - cannot automate browser
3. No screenshot API - cannot capture renders

## Testing Infrastructure Only

You can test the harness infrastructure:

```bash
cd tests/wpt
cargo test --test wpt_infrastructure
```

This tests:
- Configuration loading
- Result accumulation
- Placeholder test generation
- Report formatting

## Next Steps

1. **Add Headless Support** to browser_shell
2. **Implement WebDriver** endpoints
3. **Enable WPT Execution**

See `docs/testing/MISSING_CAPABILITIES.md` for detailed requirements.
