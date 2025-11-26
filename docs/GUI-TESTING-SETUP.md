# GUI Testing Setup Guide

This document explains how to complete the GUI-blocked features that cannot be tested in a headless environment.

## Current Status

**Verified in headless mode (879 tests passing):**
- 33 of 41 features fully verified
- HTTP connectivity to google.com and top sites confirmed
- All component unit tests passing
- All integration tests passing

**Blocked by GUI libraries (8 features):**

| Feature | Description | Status |
|---------|-------------|--------|
| FEAT-026 | WPT test harness integration | Requires browser execution |
| FEAT-027 | ACID1 test compliance | Requires visual rendering |
| FEAT-028 | Performance benchmarks | Requires running browser |
| FEAT-030 | Basic navigation - google.com | HTTP verified, GUI needed for full test |
| FEAT-031 | Performance target < 3s | Requires page load measurement |
| FEAT-032 | Memory target < 500MB | Requires running browser |
| FEAT-033 | Top 10 websites validation | HTTP verified, GUI needed for full test |
| FEAT-034 | Stability - 1 hour session | Requires running browser |

## Required System Packages

### Linux (Ubuntu/Debian)

```bash
# Install GTK3 and WebKit2GTK development libraries
sudo apt-get update
sudo apt-get install -y \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    build-essential \
    pkg-config
```

### Linux (Fedora)

```bash
sudo dnf install -y \
    gtk3-devel \
    webkit2gtk4.1-devel \
    gcc \
    pkg-config
```

### Linux (Arch)

```bash
sudo pacman -S \
    gtk3 \
    webkit2gtk-4.1 \
    base-devel \
    pkgconf
```

### macOS

```bash
# WebKit is built into macOS, but you may need:
xcode-select --install
brew install gtk+3 pkg-config
```

### Windows

WebView2 is required (bundled with Edge, usually pre-installed on Windows 10/11).

## Running GUI Tests

After installing the required packages:

```bash
# 1. Rebuild the project with GUI support
cargo build --release

# 2. Run the browser
cargo run --release

# 3. Run GUI-specific tests (when available)
cargo test --features gui

# 4. Run WPT tests
cargo test --test wpt_integration_test

# 5. Run ACID1 tests
cargo test --test acid1_test
```

## Feature Verification Steps

### FEAT-026: WPT Test Harness

1. Clone WPT repository:
   ```bash
   git clone https://github.com/web-platform-tests/wpt.git tests/wpt-suite
   ```

2. Run WPT tests:
   ```bash
   cargo test --test wpt_integration_test -- --ignored
   ```

3. Target: 40% pass rate

### FEAT-027: ACID1 Test

1. Run browser and navigate to:
   ```
   https://www.acidtests.org/
   ```

2. Or run automated test:
   ```bash
   cargo test --test acid1_test -- --ignored
   ```

### FEAT-030 & FEAT-033: Navigation Validation

1. Run the browser:
   ```bash
   cargo run
   ```

2. Navigate to google.com and verify page renders correctly

3. Test all top 10 sites and verify they load

### FEAT-031: Performance Target

1. Run performance benchmarks:
   ```bash
   cargo bench
   ```

2. Measure page load time for google.com
3. Target: Under 3 seconds

### FEAT-032: Memory Target

1. Run browser with memory profiling:
   ```bash
   /usr/bin/time -v cargo run 2>&1 | grep "Maximum resident"
   ```

2. Target: Under 500MB for single tab

### FEAT-034: Stability Validation

1. Run extended stability test:
   ```bash
   cargo test --test validation_tests test_extended_stability -- --ignored
   ```

2. Or manually run browser for 1 hour

## What's Already Done (Headless)

Even without GUI libraries, significant work has been verified:

1. **HTTP Layer Tests**
   - `test_feat_030_http_connectivity_google` - Verifies HTTP client can reach google.com
   - `test_feat_033_http_connectivity_top_10_sites` - Verifies connectivity to major sites

2. **Performance Infrastructure**
   - Timing data collection (ResourceTiming)
   - Memory-efficient message types
   - Connection pooling
   - Cache management

3. **Stability Infrastructure**
   - Component creation stability tests
   - Message bus concurrent operation tests
   - Memory leak detection tests
   - Graceful shutdown tests

4. **Navigation Infrastructure**
   - URL parsing and validation for all top 10 sites
   - Protocol detection (HTTP/HTTPS)
   - Error page generation
   - Navigation state management

## Next Steps After GUI Setup

1. Verify browser window opens correctly
2. Test navigation to google.com
3. Run WPT test suite
4. Run ACID1 test
5. Measure page load performance
6. Monitor memory usage
7. Run 1-hour stability test

## Troubleshooting

### "cannot find -lgtk-3" error
```bash
# Ubuntu/Debian
sudo apt-get install libgtk-3-dev

# Fedora
sudo dnf install gtk3-devel
```

### "cannot find -lwebkit2gtk-4.1" error
```bash
# Ubuntu/Debian
sudo apt-get install libwebkit2gtk-4.1-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel
```

### WebView2 not found (Windows)
Install Microsoft Edge or download WebView2 runtime from:
https://developer.microsoft.com/en-us/microsoft-edge/webview2/

## CI/CD Integration

For CI pipelines that need GUI testing:

```yaml
# GitHub Actions example
jobs:
  gui-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Install GUI dependencies
        run: |
          sudo apt-get update
          sudo apt-get install -y libgtk-3-dev libwebkit2gtk-4.1-dev
      - name: Build with GUI
        run: cargo build --release
      - name: Run GUI tests
        run: |
          export DISPLAY=:99
          Xvfb :99 -screen 0 1024x768x24 &
          cargo test --features gui
```
