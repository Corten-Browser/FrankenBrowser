# FrankenBrowser System Requirements

## Overview

FrankenBrowser is a Rust-based web browser using WebKit2GTK for rendering. It operates in two modes:

1. **GUI Mode** (`--features gui`): Full graphical browser with WebView rendering
2. **Headless Mode** (default): API-only mode for testing and automation

## Minimum Requirements

### All Platforms

- **Rust**: 1.70+ (with Cargo)
- **Git**: For cloning and version control

### Linux (Ubuntu/Debian)

For **GUI mode**, the following development packages are required:

```bash
# Ubuntu 24.04+ (webkit2gtk-4.1)
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    libgtk-3-dev \
    libwebkit2gtk-4.1-dev \
    libsoup-3.0-dev \
    libgdk-pixbuf-2.0-dev \
    libpango1.0-dev \
    pkg-config

# For headless GUI testing (optional)
sudo apt-get install -y xvfb
```

For **headless mode only** (no GUI, tests only):

```bash
# No additional packages needed beyond Rust toolchain
# Headless mode works without GTK/WebKit installed
```

### Windows

GUI mode requires WebView2 runtime (usually pre-installed on Windows 10+).

### macOS

GUI mode uses WKWebView (built-in on macOS).

## Building

### Headless Mode (Default)

```bash
# Build without GUI - no system dependencies needed
cargo build

# Run tests
cargo test --workspace

# All 271+ tests should pass
```

### GUI Mode

```bash
# Build with GUI features (requires system dependencies)
cargo build --features gui

# Run the browser
cargo run --features gui

# Or run with a specific URL
cargo run --features gui -- --url https://example.com
```

## Feature Flags

| Flag | Description | System Dependencies |
|------|-------------|---------------------|
| (none) | Headless mode - API only | None |
| `gui` | Full graphical browser | GTK3, WebKit2GTK |

## Verification

### Check Headless Mode

```bash
# Should work without any system dependencies
cargo test --workspace
# Expected: 271+ tests passing
```

### Check GUI Mode

```bash
# Check if GTK is available
pkg-config --exists gtk+-3.0 && echo "GTK3 available" || echo "GTK3 NOT available"

# Check if WebKit is available
pkg-config --exists webkit2gtk-4.1 && echo "WebKit available" || echo "WebKit NOT available"

# Try building with GUI
cargo build --features gui
```

## Known Issues

### Ubuntu 24.04 WebKit Version

Ubuntu 24.04 ships with `webkit2gtk-4.1`. The project uses `wry 0.53` and `tao 0.34` which support webkit2gtk-4.1 natively. If you encounter build errors:

1. Ensure you have `libwebkit2gtk-4.1-dev` (not 4.0)
2. Run `pkg-config --modversion webkit2gtk-4.1` to verify version

### Headless Testing Environments

In CI/CD or containerized environments without display:

```bash
# Use Xvfb for GUI tests
Xvfb :99 &
export DISPLAY=:99
cargo test --features gui
```

Or skip GUI tests entirely:

```bash
# Headless mode tests work without display
cargo test --workspace
```

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                      CLI Application                         │
│  (cli_app - Parses args, coordinates components)            │
└─────────────────────────────┬───────────────────────────────┘
                              │
┌─────────────────────────────▼───────────────────────────────┐
│                    Browser Shell (GUI)                       │
│  (browser_shell - WRY window, tabs, URL bar)                │
│  [Requires --features gui and system dependencies]          │
└─────────────────────────────┬───────────────────────────────┘
                              │
                        ┌─────▼──────┐
                        │ Message Bus │
                        │ (message_bus)│
                        └─────┬──────┘
         ┌────────────────────┼────────────────────┐
         │                    │                    │
   ┌─────▼─────┐       ┌─────▼──────┐      ┌─────▼──────┐
   │  Network  │       │  WebView   │      │  AdBlock   │
   │   Stack   │◄──────┤Integration │      │   Engine   │
   │(reqwest)  │       │ (wry/tao)  │      │ (adblock)  │
   └───────────┘       └────────────┘      └────────────┘
         │                    │                    │
         └────────────────────┼────────────────────┘
                              │
                       ┌──────▼──────┐
                       │Browser Core │
                       │(history,    │
                       │ bookmarks)  │
                       └─────────────┘
```

## Component Status

| Component | Headless Mode | GUI Mode |
|-----------|---------------|----------|
| shared_types | ✅ Full | ✅ Full |
| message_bus | ✅ Full | ✅ Full |
| config_manager | ✅ Full | ✅ Full |
| network_stack | ✅ Full | ✅ Full |
| adblock_engine | ✅ Full | ✅ Full |
| browser_core | ✅ Full | ✅ Full |
| webview_integration | ⚠️ Stub | ✅ Full |
| browser_shell | ⚠️ Stub | ✅ Full |
| webdriver | ✅ Full | ✅ Full |
| cli_app | ✅ Full | ✅ Full |

**Note**: In headless mode, webview_integration and browser_shell use stub implementations that track state but don't render. Full rendering requires GUI mode.
