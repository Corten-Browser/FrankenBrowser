# Frankenstein Browser Component Specification
## Phase 1 Minimal Viable Browser Implementation
### Part of CortenBrowser System Architecture

### Component Overview

**Component Name**: frankenstein-browser  
**Version**: 1.0.0  
**System Project**: CortenBrowser  
**Purpose**: Minimal viable browser combining existing Rust components for rapid prototype  
**Development Timeline**: 3 weeks  
**Lines of Code Estimate**: 15,000-20,000 (primarily integration code)  
**Token Budget**: ~50,000 tokens (well within Claude Code limits)  
**Test Coverage Target**: WPT 40% overall using existing components  

### Architecture Design

The Frankenstein Browser uses a hybrid approach leveraging mature existing components with minimal custom integration code. The architecture prioritizes rapid deployment while establishing the foundational message bus system for future component replacement.

```
┌─────────────────────────────────────────────────────────┐
│                    Browser Shell (WRY)                  │
│  ┌─────────────┐  ┌──────────────┐  ┌──────────────┐  │
│  │   Tab Mgr   │  │  URL Bar     │  │   Controls   │  │
│  └─────────────┘  └──────────────┘  └──────────────┘  │
└─────────────────────────┬───────────────────────────────┘
                          │
                    ┌─────▼──────┐
                    │ Message Bus │
                    └─────┬──────┘
         ┌────────────────┼────────────────┐
         │                │                │
    ┌────▼────┐    ┌─────▼─────┐   ┌─────▼─────┐
    │ Network │    │  WebView  │   │ Ad Blocker│
    │  Stack  │◄───┤   Core    │   │  Engine   │
    └─────────┘    └───────────┘   └───────────┘
```

### External Dependencies

```toml
[dependencies]
# Core browser shell
wry = "0.35"
tao = "0.25"  # Window management

# Networking
reqwest = { version = "0.11", features = ["blocking", "cookies", "gzip", "brotli", "json"] }
url = "2.5"
cookie_store = "0.20"

# Ad blocking
adblock = "0.8"

# WebView core
webkit2gtk = { version = "2.0", features = ["v2_38"] }  # Linux
webview2-com = "0.28"  # Windows
cocoa = "0.25"  # macOS

# Async runtime
tokio = { version = "1.35", features = ["full"] }

# UI framework
egui = "0.24"
eframe = "0.24"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"

# Error handling
anyhow = "1.0"
thiserror = "1.0"

# Logging
tracing = "0.1"
tracing-subscriber = "0.3"

# Database for history/bookmarks
rusqlite = { version = "0.30", features = ["bundled"] }

# IPC for component communication
crossbeam-channel = "0.5"
```

### Project Structure

```
frankenstein-browser/
├── Cargo.toml
├── build.rs
├── README.md
├── src/
│   ├── main.rs
│   ├── lib.rs
│   ├── shell/
│   │   ├── mod.rs
│   │   ├── window.rs
│   │   ├── tab_manager.rs
│   │   ├── ui_components.rs
│   │   └── menu.rs
│   ├── core/
│   │   ├── mod.rs
│   │   ├── browser_engine.rs
│   │   ├── navigation.rs
│   │   ├── history.rs
│   │   └── bookmarks.rs
│   ├── network/
│   │   ├── mod.rs
│   │   ├── http_client.rs
│   │   ├── request_handler.rs
│   │   ├── cache.rs
│   │   └── cookies.rs
│   ├── adblock/
│   │   ├── mod.rs
│   │   ├── filter_engine.rs
│   │   ├── rule_loader.rs
│   │   └── element_hider.rs
│   ├── webview/
│   │   ├── mod.rs
│   │   ├── webview_wrapper.rs
│   │   ├── javascript_bridge.rs
│   │   └── platform/
│   │       ├── mod.rs
│   │       ├── linux.rs
│   │       ├── windows.rs
│   │       └── macos.rs
│   ├── message_bus/
│   │   ├── mod.rs
│   │   ├── messages.rs
│   │   ├── dispatcher.rs
│   │   └── handlers.rs
│   ├── config/
│   │   ├── mod.rs
│   │   ├── settings.rs
│   │   └── defaults.rs
│   └── utils/
│       ├── mod.rs
│       ├── logging.rs
│       └── errors.rs
├── resources/
│   ├── icons/
│   ├── filters/
│   │   └── easylist.txt
│   └── config/
│       └── default_settings.toml
└── tests/
    ├── integration/
    └── unit/
```

### Core Implementation Files

#### src/main.rs
```rust
use anyhow::Result;
use frankenstein_browser::{BrowserApp, Config};
use tracing_subscriber;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt::init();
    
    // Load configuration
    let config = Config::load_or_default()?;
    
    // Create and run browser application
    let app = BrowserApp::new(config)?;
    app.run()
}
```

#### src/lib.rs
```rust
pub mod shell;
pub mod core;
pub mod network;
pub mod adblock;
pub mod webview;
pub mod message_bus;
pub mod config;
pub mod utils;

use anyhow::Result;
use std::sync::Arc;
use tokio::runtime::Runtime;

pub use config::Config;

pub struct BrowserApp {
    runtime: Arc<Runtime>,
    shell: shell::BrowserShell,
    message_bus: message_bus::MessageBus,
    network: network::NetworkStack,
    adblock: adblock::AdBlockEngine,
}

impl BrowserApp {
    pub fn new(config: Config) -> Result<Self> {
        let runtime = Arc::new(Runtime::new()?);
        let message_bus = message_bus::MessageBus::new();
        
        let network = network::NetworkStack::new(
            config.network_config(),
            message_bus.sender(),
        )?;
        
        let adblock = adblock::AdBlockEngine::new(
            config.adblock_config(),
            message_bus.sender(),
        )?;
        
        let shell = shell::BrowserShell::new(
            config.shell_config(),
            message_bus.sender(),
            runtime.clone(),
        )?;
        
        Ok(Self {
            runtime,
            shell,
            message_bus,
            network,
            adblock,
        })
    }
    
    pub fn run(mut self) -> Result<()> {
        // Start message bus processing
        self.message_bus.start()?;
        
        // Initialize components
        self.network.initialize()?;
        self.adblock.initialize()?;
        
        // Run the browser shell (blocks until exit)
        self.shell.run()?;
        
        // Cleanup
        self.message_bus.shutdown()?;
        Ok(())
    }
}
```

#### src/message_bus/messages.rs
```rust
use serde::{Deserialize, Serialize};
use url::Url;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BrowserMessage {
    // Navigation messages
    NavigateRequest { tab_id: u32, url: Url },
    NavigateResponse { tab_id: u32, content: Vec<u8> },
    NavigateError { tab_id: u32, error: String },
    
    // Network messages
    HttpRequest { request_id: u64, url: Url, headers: HashMap<String, String> },
    HttpResponse { request_id: u64, status: u16, body: Vec<u8> },
    
    // Ad blocking messages
    ShouldBlock { url: Url, resource_type: ResourceType },
    BlockDecision { block: bool, reason: Option<String> },
    
    // Tab management
    CreateTab { parent_window: u32 },
    CloseTab { tab_id: u32 },
    SwitchTab { tab_id: u32 },
    
    // Browser control
    Shutdown,
    Reload { tab_id: u32 },
    GoBack { tab_id: u32 },
    GoForward { tab_id: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Document,
    Script,
    Image,
    Stylesheet,
    Font,
    Media,
    Websocket,
    Xhr,
    Other,
}
```

#### src/shell/window.rs
```rust
use wry::{
    application::{
        event_loop::{ControlFlow, EventLoop},
        window::WindowBuilder,
    },
    webview::WebViewBuilder,
};
use crate::message_bus::MessageSender;
use anyhow::Result;

pub struct BrowserWindow {
    event_loop: EventLoop<()>,
    webview: wry::webview::WebView,
    message_sender: MessageSender,
}

impl BrowserWindow {
    pub fn new(message_sender: MessageSender) -> Result<Self> {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("Frankenstein Browser")
            .with_inner_size(wry::application::dpi::LogicalSize::new(1280, 720))
            .build(&event_loop)?;
        
        let webview = WebViewBuilder::new(window)?
            .with_url("https://www.google.com")?
            .with_devtools(true)
            .with_initialization_script(include_str!("../../resources/init.js"))
            .with_ipc_handler(move |_window, message| {
                // Handle IPC messages from webview
                handle_ipc_message(message, &message_sender);
            })
            .build()?;
        
        Ok(Self {
            event_loop,
            webview,
            message_sender,
        })
    }
    
    pub fn run(self) -> Result<()> {
        self.event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;
            
            match event {
                // Handle window events
                _ => {}
            }
        });
    }
}

fn handle_ipc_message(message: String, sender: &MessageSender) {
    // Parse and route IPC messages
}
```

#### src/network/http_client.rs
```rust
use reqwest::{Client, Response};
use std::collections::HashMap;
use url::Url;
use crate::message_bus::{MessageSender, BrowserMessage, ResourceType};
use anyhow::Result;

pub struct HttpClient {
    client: Client,
    message_sender: MessageSender,
}

impl HttpClient {
    pub fn new(message_sender: MessageSender) -> Result<Self> {
        let client = Client::builder()
            .user_agent("FrankensteinBrowser/1.0")
            .cookie_store(true)
            .gzip(true)
            .brotli(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()?;
        
        Ok(Self {
            client,
            message_sender,
        })
    }
    
    pub async fn fetch(&self, url: Url) -> Result<Vec<u8>> {
        // Check with ad blocker first
        self.message_sender.send(BrowserMessage::ShouldBlock {
            url: url.clone(),
            resource_type: ResourceType::Document,
        })?;
        
        // Wait for response (simplified - real impl needs proper async handling)
        
        let response = self.client.get(url.as_str()).send().await?;
        let body = response.bytes().await?.to_vec();
        
        Ok(body)
    }
}
```

#### src/adblock/filter_engine.rs
```rust
use adblock::engine::Engine;
use adblock::request::Request;
use crate::message_bus::{MessageSender, BrowserMessage, ResourceType};
use anyhow::Result;

pub struct FilterEngine {
    engine: Engine,
    message_sender: MessageSender,
}

impl FilterEngine {
    pub fn new(message_sender: MessageSender) -> Result<Self> {
        let mut engine = Engine::new(true);
        
        // Load EasyList
        let filters = std::fs::read_to_string("resources/filters/easylist.txt")?;
        engine.add_filters(&filters);
        
        Ok(Self {
            engine,
            message_sender,
        })
    }
    
    pub fn should_block(&self, url: &str, resource_type: ResourceType) -> bool {
        let request = Request::new(url, "", &resource_type_to_string(resource_type));
        let result = self.engine.check_request(&request);
        result.matched
    }
}

fn resource_type_to_string(rt: ResourceType) -> &'static str {
    match rt {
        ResourceType::Script => "script",
        ResourceType::Image => "image",
        ResourceType::Stylesheet => "stylesheet",
        ResourceType::Document => "document",
        _ => "other",
    }
}
```

### Configuration System

#### resources/config/default_settings.toml
```toml
[browser]
homepage = "https://www.google.com"
enable_devtools = true
default_search_engine = "google"

[network]
max_connections_per_host = 6
timeout_seconds = 30
enable_cookies = true
enable_cache = true
cache_size_mb = 500

[adblock]
enabled = true
update_filters_on_startup = false
custom_filters = []

[privacy]
do_not_track = true
clear_cookies_on_exit = false
block_third_party_cookies = false

[appearance]
theme = "auto"  # "light", "dark", "auto"
default_zoom = 1.0
```

### Build Configuration

#### Cargo.toml
```toml
[package]
name = "frankenstein-browser"
version = "1.0.0"
edition = "2021"
authors = ["Browser Team"]
license = "MIT OR Apache-2.0"

[dependencies]
wry = "0.35"
tao = "0.25"
reqwest = { version = "0.11", features = ["blocking", "cookies", "gzip", "brotli", "json", "stream"] }
url = "2.5"
cookie_store = "0.20"
adblock = "0.8"
tokio = { version = "1.35", features = ["full"] }
egui = "0.24"
eframe = "0.24"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
toml = "0.8"
anyhow = "1.0"
thiserror = "1.0"
tracing = "0.1"
tracing-subscriber = "0.3"
rusqlite = { version = "0.30", features = ["bundled"] }
crossbeam-channel = "0.5"
directories = "5.0"
once_cell = "1.19"

[target.'cfg(target_os = "linux")'.dependencies]
webkit2gtk = { version = "2.0", features = ["v2_38"] }
gtk = "0.18"

[target.'cfg(target_os = "windows")'.dependencies]
webview2-com = "0.28"
windows = "0.52"

[target.'cfg(target_os = "macos")'.dependencies]
cocoa = "0.25"
objc = "0.2"

[build-dependencies]
winres = "0.1"

[profile.release]
opt-level = 3
lto = true
codegen-units = 1
strip = true

[profile.dev]
opt-level = 0
debug = true
```

#### build.rs
```rust
fn main() {
    // Windows-specific build configuration
    #[cfg(target_os = "windows")]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("resources/icons/browser.ico");
        res.compile().unwrap();
    }
    
    // Download EasyList if not present
    if !std::path::Path::new("resources/filters/easylist.txt").exists() {
        download_easylist();
    }
}

fn download_easylist() {
    // Download logic here
    println!("cargo:warning=Downloading EasyList filters...");
}
```

### Implementation Milestones

#### Week 1: Core Infrastructure
1. **Day 1-2**: Project setup and dependency configuration
2. **Day 3-4**: Message bus implementation and testing
3. **Day 5-7**: Basic WRY window with tab management

**Validation**: Window opens, tabs can be created/closed

#### Week 2: Networking and Content
1. **Day 8-9**: HTTP client with request/response handling
2. **Day 10-11**: Ad blocker integration
3. **Day 12-14**: WebView integration and navigation

**Validation**: Can navigate to websites, content loads

#### Week 3: Polish and Features
1. **Day 15-16**: History and bookmarks
2. **Day 17-18**: Settings persistence
3. **Day 19-21**: Testing and bug fixes

**Validation**: Passes acceptance criteria

### Test Strategy

#### Unit Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_message_bus_routing() {
        let bus = MessageBus::new();
        let sender = bus.sender();
        sender.send(BrowserMessage::Shutdown).unwrap();
        // Verify message received
    }
    
    #[test]
    fn test_adblock_filter() {
        let engine = FilterEngine::new(mock_sender()).unwrap();
        assert!(engine.should_block(
            "https://doubleclick.net/ad.js",
            ResourceType::Script
        ));
    }
    
    #[tokio::test]
    async fn test_http_client() {
        let client = HttpClient::new(mock_sender()).unwrap();
        let content = client.fetch("https://example.com".parse().unwrap()).await;
        assert!(content.is_ok());
    }
}
```

#### Integration Tests
```rust
#[test]
fn test_browser_startup() {
    let config = Config::default();
    let app = BrowserApp::new(config).unwrap();
    // Verify all components initialize
}

#[test]
fn test_navigation_flow() {
    // Test complete navigation from URL bar to rendered content
}
```

### Public Test Suite Integration

The Frankenstein Browser, as Phase 1 of the CortenBrowser project, integrates with established web platform test suites to ensure compatibility and correctness. While full test suite compliance is targeted for later phases, Phase 1 establishes the test harness infrastructure.

#### Web Platform Tests (WPT) Integration

```toml
# tests/wpt-config.toml
[wpt]
test_path = "../wpt"
binary_path = "./target/release/frankenstein-browser"
test_subsets = [
    "html/browsers/browsing-the-web/navigating-across-documents",
    "html/browsers/history",
    "fetch/api/basic"
]
expected_pass_rate = 0.40  # 40% for Phase 1 with existing components
```

##### WPT Test Harness
```rust
// tests/wpt_harness.rs
use wpt_runner::{TestHarness, TestResult};

pub struct FrankensteinTestHarness {
    browser: BrowserApp,
}

impl TestHarness for FrankensteinTestHarness {
    fn navigate(&mut self, url: &str) -> TestResult {
        self.browser.navigate(url)?;
        TestResult::from_dom(self.browser.get_dom())
    }
    
    fn execute_script(&mut self, script: &str) -> TestResult {
        // Phase 1: Use WebView's JS engine
        self.browser.webview.execute_script(script)
    }
    
    fn get_resource_timing(&self) -> Vec<ResourceTiming> {
        self.browser.network.get_timing_data()
    }
}

// Run subset of WPT tests appropriate for Phase 1
pub fn run_phase1_wpt_tests() -> TestResults {
    let harness = FrankensteinTestHarness::new();
    let tests = vec![
        "navigation/basic.html",
        "fetch/basic-fetch.html",
        "history/pushstate.html",
    ];
    
    wpt_runner::run_tests(harness, tests)
}
```

#### ACID Test Compliance

```rust
// tests/acid_tests.rs
#[test]
fn test_acid1_compliance() {
    let browser = setup_browser();
    browser.navigate("file:///tests/acid1.html");
    
    // ACID1 tests basic CSS1 box model
    let result = browser.render_and_compare("acid1_reference.png");
    assert!(result.similarity > 0.99, "ACID1 must pass in Phase 1");
}

// ACID2 and ACID3 tests are marked for later phases
#[test]
#[ignore]  // Enable in Phase 2
fn test_acid2_compliance() {
    // Tests CSS2.1 compliance
}
```

#### Performance Benchmarks

```rust
// benchmarks/performance.rs
use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn benchmark_page_load(c: &mut Criterion) {
    let browser = setup_browser();
    
    c.bench_function("load_simple_page", |b| {
        b.iter(|| {
            browser.navigate(black_box("https://example.com"));
            browser.wait_for_load();
        })
    });
}

fn benchmark_tab_switching(c: &mut Criterion) {
    let mut browser = setup_browser_with_tabs(5);
    
    c.bench_function("switch_tabs", |b| {
        b.iter(|| {
            browser.switch_to_tab(black_box(2));
        })
    });
}

criterion_group!(benches, benchmark_page_load, benchmark_tab_switching);
criterion_main!(benches);
```

#### Test Infrastructure Setup

```yaml
# .github/workflows/frankenstein-tests.yml
name: Frankenstein Browser Tests

on: [push, pull_request]

jobs:
  unit-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - uses: actions-rs/toolchain@v1
      - name: Run unit tests
        run: cargo test --all

  wpt-phase1:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Clone WPT
        run: |
          git clone --depth=1 https://github.com/web-platform-tests/wpt.git
          
      - name: Build Frankenstein Browser
        run: cargo build --release
        
      - name: Run Phase 1 WPT subset
        run: |
          cd wpt
          ./wpt run --binary=../target/release/frankenstein-browser \
                    --metadata=../tests/wpt-metadata \
                    --include=../tests/phase1-tests.txt
          
      - name: Check pass rate
        run: |
          python3 tests/check_pass_rate.py --min-rate=0.40

  performance-tests:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      - name: Run benchmarks
        run: |
          cargo bench --bench performance
          
      - name: Compare with baseline
        run: |
          python3 tests/compare_benchmarks.py \
                  --baseline=tests/performance-baseline.json \
                  --max-regression=10
```

#### Test Result Tracking

```sql
-- tests/schema.sql
CREATE TABLE test_runs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    component TEXT NOT NULL,
    test_suite TEXT NOT NULL,
    total_tests INTEGER NOT NULL,
    passed_tests INTEGER NOT NULL,
    failed_tests INTEGER NOT NULL,
    skipped_tests INTEGER NOT NULL,
    pass_rate REAL NOT NULL,
    commit_hash TEXT NOT NULL,
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE performance_results (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    benchmark TEXT NOT NULL,
    mean_time_ns INTEGER NOT NULL,
    std_dev_ns INTEGER NOT NULL,
    min_time_ns INTEGER NOT NULL,
    max_time_ns INTEGER NOT NULL,
    commit_hash TEXT NOT NULL,
    run_timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);
```

#### Test Monitoring Dashboard

```rust
// tests/dashboard.rs
use rusqlite::Connection;
use serde::Serialize;

#[derive(Serialize)]
struct TestMetrics {
    wpt_pass_rate: f64,
    acid1_pass: bool,
    avg_page_load_ms: f64,
    memory_usage_mb: f64,
    blocked_ads_count: u64,
}

pub fn generate_test_report(db_path: &str) -> TestMetrics {
    let conn = Connection::open(db_path).unwrap();
    
    let wpt_pass_rate: f64 = conn.query_row(
        "SELECT AVG(pass_rate) FROM test_runs 
         WHERE test_suite = 'wpt' AND run_timestamp > datetime('now', '-7 days')",
        [],
        |row| row.get(0)
    ).unwrap_or(0.0);
    
    TestMetrics {
        wpt_pass_rate,
        acid1_pass: check_acid1_status(&conn),
        avg_page_load_ms: get_avg_page_load(&conn),
        memory_usage_mb: get_avg_memory_usage(&conn),
        blocked_ads_count: get_blocked_ads_count(&conn),
    }
}

### Build Instructions for Claude Code

#### Step 1: Environment Setup
```bash
# Create project directory
mkdir frankenstein-browser && cd frankenstein-browser

# Initialize Rust project
cargo init --lib

# Copy the Cargo.toml content provided above
```

#### Step 2: Create Directory Structure
```bash
# Create all directories as specified in project structure
mkdir -p src/{shell,core,network,adblock,webview,message_bus,config,utils}
mkdir -p src/webview/platform
mkdir -p resources/{icons,filters,config}
mkdir -p tests/{integration,unit}
```

#### Step 3: Implement Core Files
1. Copy provided code snippets to respective files
2. Implement remaining modules following the patterns established
3. Ensure all imports and dependencies are resolved

#### Step 4: Download Resources
```bash
# Download EasyList
wget -O resources/filters/easylist.txt \
  https://easylist.to/easylist/easylist.txt

# Create default settings
cp default_settings.toml resources/config/
```

#### Step 5: Build and Test
```bash
# Build the project
cargo build

# Run tests
cargo test

# Run the browser
cargo run
```

### Platform-Specific Notes

#### Linux Requirements
```bash
# Ubuntu/Debian
sudo apt-get install libwebkit2gtk-4.1-dev libgtk-3-dev

# Fedora
sudo dnf install webkit2gtk4.1-devel gtk3-devel
```

#### Windows Requirements
- Windows 10/11 with WebView2 runtime installed
- Visual Studio 2022 or Build Tools

#### macOS Requirements
- Xcode Command Line Tools
- macOS 10.15 or later

### Success Criteria

#### Phase 1 Test-Driven Metrics
1. **Basic Navigation**: Browser can load and display google.com
2. **Ad Blocking**: Blocks ads on major websites (easylist compliance)
3. **Tab Management**: Can open, switch, and close tabs
4. **Performance**: Page loads within 3 seconds on broadband
5. **Stability**: No crashes during 1-hour usage session

#### Phase 1 Test Suite Targets
- **ACID1**: PASS (required)
- **WPT Overall**: 40% pass rate (using existing components)
- **Top 10 Websites**: 100% load successfully
- **Memory Usage**: < 500MB for single tab
- **Ad Block Effectiveness**: > 90% of EasyList rules applied

#### Test Progression Path
- Week 1: Unit tests 100% pass
- Week 2: Integration tests 100% pass  
- Week 3: WPT subset 40% pass, ACID1 pass

### Component Integration Points

The Frankenstein Browser establishes these integration points for future components:

1. **Message Bus**: All future components will connect here
2. **WebView Bridge**: JavaScript injection point for DOM manipulation
3. **Network Interceptor**: Hook for custom protocol handlers
4. **Extension Points**: Prepared for extension system integration

### Known Limitations

1. No JavaScript engine control (uses system WebView)
2. Limited CSS inspection capabilities
3. No DevTools Protocol implementation
4. Basic cookie management only
5. No service worker support

### Future Migration Path

This implementation prepares for migration to pure Rust components:

1. **WebView → Rust Renderer**: Replace WRY with custom renderer
2. **System JS → Rust JS**: Swap WebView JS for rusty_v8 or Boa
3. **Basic Network → Full Stack**: Enhance with HTTP/2, QUIC
4. **Simple UI → Full Chrome**: Replace with egui/iced implementation

### Monitoring and Metrics

```rust
use std::time::Duration;

pub struct BrowserMetrics {
    pub page_load_time: Duration,
    pub memory_usage: usize,
    pub active_tabs: u32,
    pub blocked_requests: u64,
    pub network_requests: u64,
}
```

### Error Handling Strategy

All errors bubble up through `anyhow::Result` with context:

```rust
use anyhow::{Context, Result};

fn load_page(url: &str) -> Result<Vec<u8>> {
    fetch_content(url)
        .context("Failed to fetch page content")?;
    parse_html(content)
        .context("Failed to parse HTML")?;
    Ok(rendered)
}
```

### Security Considerations

1. **Process Isolation**: Each tab runs in system WebView sandbox
2. **HTTPS Enforcement**: Warn on non-HTTPS content
3. **CSP Respect**: Honor Content Security Policy headers
4. **Cookie Isolation**: Per-origin cookie storage
5. **Local Storage**: Encrypted settings database

### Claude Code Implementation Commands

For Claude Code to build this independently, use these commands in sequence:

```bash
# 1. Setup and structure
cargo new frankenstein-browser --lib
cd frankenstein-browser

# 2. Create complete directory structure
mkdir -p src/{shell,core,network,adblock,webview/{platform},message_bus,config,utils}
mkdir -p resources/{icons,filters,config}
mkdir -p tests/{integration,unit,wpt,benchmarks}

# 3. Install system dependencies (Linux)
sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev

# 4. Clone test suites
git clone --depth=1 https://github.com/web-platform-tests/wpt.git ../wpt

# 5. Implement all source files
# [Claude Code implements all files based on specifications]

# 6. Download filter lists
curl -o resources/filters/easylist.txt https://easylist.to/easylist/easylist.txt

# 7. Build and verify
cargo build --release
cargo test

# 8. Run WPT subset
cd ../wpt
./wpt run --binary=../frankenstein-browser/target/release/frankenstein-browser \
          --include=../frankenstein-browser/tests/phase1-tests.txt

# 9. Run the browser
cd ../frankenstein-browser
cargo run --release
```

### Test-Driven Development Workflow

When implementing features, Claude Code should follow this TDD cycle:

```rust
// Step 1: Write failing test
#[test]
fn test_new_feature() {
    let browser = setup_browser();
    let result = browser.new_feature();
    assert_eq!(result, expected_value);
}

// Step 2: Implement minimal code to pass
impl Browser {
    fn new_feature(&self) -> Result<Value> {
        // Minimal implementation
    }
}

// Step 3: Refactor with confidence
// Step 4: Add integration test
// Step 5: Run relevant WPT tests
```

### Claude Code Test-First Prompts

For optimal results with Claude Code, use these structured prompts:

```
"Implement [feature] for the Frankenstein Browser using TDD:

1. First write a failing unit test in tests/unit/[feature]_test.rs
2. Implement the minimal code to make the test pass
3. Add comprehensive test cases for edge conditions
4. Write an integration test that uses the feature end-to-end
5. Identify and run relevant WPT tests from the phase1 subset
6. Ensure no regression in existing tests

Target metrics:
- Unit test coverage: 90%
- Integration test pass: 100%
- WPT subset pass rate: maintain 40% minimum"
```

### Validation Test Suite

```rust
// tests/integration/validation.rs
use frankenstein_browser::BrowserApp;
use std::time::Duration;

#[test]
fn validate_google_loads() {
    let mut browser = BrowserApp::new(Default::default()).unwrap();
    browser.navigate("https://www.google.com").unwrap();
    browser.wait_for_load(Duration::from_secs(10)).unwrap();
    
    let title = browser.get_page_title().unwrap();
    assert!(title.contains("Google"));
}

#[test]
fn validate_ad_blocking() {
    let mut browser = BrowserApp::new(Default::default()).unwrap();
    browser.navigate("https://www.cnn.com").unwrap();
    browser.wait_for_load(Duration::from_secs(10)).unwrap();
    
    let metrics = browser.get_metrics();
    assert!(metrics.blocked_requests > 0, "Should block some ads");
}

#[test]
fn validate_tab_management() {
    let mut browser = BrowserApp::new(Default::default()).unwrap();
    
    // Create 3 tabs
    let tab1 = browser.create_tab().unwrap();
    let tab2 = browser.create_tab().unwrap();
    let tab3 = browser.create_tab().unwrap();
    
    // Switch between them
    browser.switch_to_tab(tab1).unwrap();
    browser.switch_to_tab(tab3).unwrap();
    
    // Close middle tab
    browser.close_tab(tab2).unwrap();
    
    // Verify correct tab count
    assert_eq!(browser.get_tab_count(), 2);
}

#[test]
fn validate_acid1_rendering() {
    let mut browser = BrowserApp::new(Default::default()).unwrap();
    browser.navigate("file:///tests/fixtures/acid1.html").unwrap();
    browser.wait_for_load(Duration::from_secs(5)).unwrap();
    
    let screenshot = browser.capture_screenshot().unwrap();
    let reference = image::open("tests/fixtures/acid1_reference.png").unwrap();
    
    let similarity = compare_images(&screenshot, &reference);
    assert!(similarity > 0.99, "ACID1 test must pass with >99% accuracy");
}
```

### Continuous Integration Pipeline

```yaml
# tests/ci/validation-pipeline.yml
name: Frankenstein Validation Pipeline

stages:
  - build
  - test
  - benchmark
  - deploy

build:
  stage: build
  script:
    - cargo build --release
    - cargo test --no-run
  artifacts:
    paths:
      - target/release/frankenstein-browser

unit-tests:
  stage: test
  script:
    - cargo test --lib
    - cargo test --doc

integration-tests:
  stage: test
  script:
    - cargo test --test integration

wpt-tests:
  stage: test
  script:
    - ./scripts/run-wpt-phase1.sh
  allow_failure: true  # WPT is informational in Phase 1

benchmarks:
  stage: benchmark
  script:
    - cargo bench
    - ./scripts/check-performance-regression.sh

package:
  stage: deploy
  script:
    - cargo package
    - ./scripts/create-release.sh
  only:
    - tags
```

---

This specification provides a complete implementation blueprint for the Frankenstein Browser that Claude Code can execute independently. The component serves as Phase 1 of the CortenBrowser project, establishing the foundation for a larger competitive browser while being immediately functional as a basic web browser with comprehensive test validation.