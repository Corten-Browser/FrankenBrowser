# FrankenBrowser Architecture Plan

## Component Overview

### Dependency Levels

**Level 0: Base Components (No Dependencies)**
1. **shared_types** (~5,000 tokens, ~500 LOC)
   - Common types, enums, messages
   - Resource types, error types
   - No dependencies

2. **message_bus** (~8,000 tokens, ~800 LOC)
   - Message bus implementation
   - Dispatcher and handlers
   - Channel-based communication
   - Depends on: shared_types

**Level 1: Core Components**
3. **config_manager** (~6,000 tokens, ~600 LOC)
   - Configuration loading/saving
   - Settings management
   - Defaults handling
   - Depends on: shared_types

4. **network_stack** (~12,000 tokens, ~1,200 LOC)
   - HTTP client with reqwest
   - Request/response handling
   - Cache management
   - Cookie handling
   - Depends on: shared_types, message_bus

5. **adblock_engine** (~8,000 tokens, ~800 LOC)
   - Filter engine using adblock crate
   - Rule loading from EasyList
   - Element hiding logic
   - Depends on: shared_types, message_bus

**Level 2: Feature Components**
6. **browser_core** (~10,000 tokens, ~1,000 LOC)
   - Browser engine coordination
   - Navigation logic
   - History management
   - Bookmarks management
   - Depends on: shared_types, message_bus, network_stack, config_manager

7. **webview_integration** (~12,000 tokens, ~1,200 LOC)
   - WebView wrapper (wry)
   - JavaScript bridge
   - Platform-specific implementations (Linux/Windows/macOS)
   - IPC handling
   - Depends on: shared_types, message_bus

**Level 3: Integration Component**
8. **browser_shell** (~14,000 tokens, ~1,400 LOC)
   - Window management (tao)
   - Tab management
   - UI components
   - Menu system
   - Coordinates all components
   - Depends on: all core and feature components

**Level 4: Application Component**
9. **cli_app** (~3,000 tokens, ~300 LOC)
   - Main entry point
   - Application initialization
   - Component wiring
   - Runtime management
   - Depends on: browser_shell

**Total Estimated Size:** ~78,000 tokens (~7,800 LOC)

## Component Directory Structure

```
components/
├── shared_types/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── messages.rs
│   │   ├── resource_types.rs
│   │   └── errors.rs
│   └── tests/
│
├── message_bus/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── bus.rs
│   │   ├── dispatcher.rs
│   │   └── handlers.rs
│   └── tests/
│
├── config_manager/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── settings.rs
│   │   └── defaults.rs
│   └── tests/
│
├── network_stack/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── http_client.rs
│   │   ├── request_handler.rs
│   │   ├── cache.rs
│   │   └── cookies.rs
│   └── tests/
│
├── adblock_engine/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── filter_engine.rs
│   │   ├── rule_loader.rs
│   │   └── element_hider.rs
│   └── tests/
│
├── browser_core/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── engine.rs
│   │   ├── navigation.rs
│   │   ├── history.rs
│   │   └── bookmarks.rs
│   └── tests/
│
├── webview_integration/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── webview_wrapper.rs
│   │   ├── javascript_bridge.rs
│   │   └── platform/
│   │       ├── mod.rs
│   │       ├── linux.rs
│   │       ├── windows.rs
│   │       └── macos.rs
│   └── tests/
│
├── browser_shell/
│   ├── CLAUDE.md
│   ├── README.md
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── window.rs
│   │   ├── tab_manager.rs
│   │   ├── ui_components.rs
│   │   └── menu.rs
│   └── tests/
│
└── cli_app/
    ├── CLAUDE.md
    ├── README.md
    ├── Cargo.toml
    ├── src/
    │   ├── main.rs
    │   └── lib.rs
    └── tests/
```

## Build Order (Topological Sort)

1. **Wave 1** (parallel): shared_types
2. **Wave 2** (parallel): message_bus, config_manager
3. **Wave 3** (parallel): network_stack, adblock_engine
4. **Wave 4** (parallel): browser_core, webview_integration
5. **Wave 5** (sequential): browser_shell
6. **Wave 6** (sequential): cli_app

## API Contracts

### shared_types → All Components
```rust
// BrowserMessage enum
pub enum BrowserMessage {
    NavigateRequest { tab_id: u32, url: Url },
    NavigateResponse { tab_id: u32, content: Vec<u8> },
    NavigateError { tab_id: u32, error: String },
    HttpRequest { request_id: u64, url: Url, headers: HashMap<String, String> },
    HttpResponse { request_id: u64, status: u16, body: Vec<u8> },
    ShouldBlock { url: Url, resource_type: ResourceType },
    BlockDecision { block: bool, reason: Option<String> },
    CreateTab { parent_window: u32 },
    CloseTab { tab_id: u32 },
    SwitchTab { tab_id: u32 },
    Shutdown,
    Reload { tab_id: u32 },
    GoBack { tab_id: u32 },
    GoForward { tab_id: u32 },
}

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

### message_bus → All Active Components
```rust
pub trait MessageSender: Send + Sync {
    fn send(&self, message: BrowserMessage) -> Result<()>;
}

pub trait MessageHandler: Send + Sync {
    fn handle(&self, message: BrowserMessage) -> Result<()>;
}

pub struct MessageBus {
    // Internal implementation
}

impl MessageBus {
    pub fn new() -> Self;
    pub fn sender(&self) -> Box<dyn MessageSender>;
    pub fn register_handler(&mut self, handler: Box<dyn MessageHandler>);
    pub fn start(&mut self) -> Result<()>;
    pub fn shutdown(&mut self) -> Result<()>;
}
```

### config_manager API
```rust
pub struct Config {
    // Fields from specification
}

impl Config {
    pub fn load_or_default() -> Result<Self>;
    pub fn load_from_file(path: &Path) -> Result<Self>;
    pub fn save_to_file(&self, path: &Path) -> Result<()>;
    pub fn network_config(&self) -> NetworkConfig;
    pub fn adblock_config(&self) -> AdBlockConfig;
    pub fn shell_config(&self) -> ShellConfig;
}
```

### network_stack API
```rust
pub struct NetworkStack {
    // Internal implementation
}

impl NetworkStack {
    pub fn new(config: NetworkConfig, sender: Box<dyn MessageSender>) -> Result<Self>;
    pub fn initialize(&mut self) -> Result<()>;
    pub async fn fetch(&self, url: Url) -> Result<Vec<u8>>;
    pub fn get_timing_data(&self) -> Vec<ResourceTiming>;
}
```

### adblock_engine API
```rust
pub struct AdBlockEngine {
    // Internal implementation
}

impl AdBlockEngine {
    pub fn new(config: AdBlockConfig, sender: Box<dyn MessageSender>) -> Result<Self>;
    pub fn initialize(&mut self) -> Result<()>;
    pub fn should_block(&self, url: &str, resource_type: ResourceType) -> bool;
}
```

### browser_core API
```rust
pub struct BrowserEngine {
    // Internal implementation
}

impl BrowserEngine {
    pub fn new(
        config: Config,
        network: NetworkStack,
        message_bus: Box<dyn MessageSender>
    ) -> Result<Self>;

    pub fn navigate(&mut self, tab_id: u32, url: Url) -> Result<()>;
    pub fn go_back(&mut self, tab_id: u32) -> Result<()>;
    pub fn go_forward(&mut self, tab_id: u32) -> Result<()>;
    pub fn reload(&mut self, tab_id: u32) -> Result<()>;

    pub fn add_bookmark(&mut self, url: Url, title: String) -> Result<()>;
    pub fn get_bookmarks(&self) -> Vec<Bookmark>;
    pub fn get_history(&self) -> Vec<HistoryEntry>;
}
```

### webview_integration API
```rust
pub struct WebViewWrapper {
    // Internal implementation
}

impl WebViewWrapper {
    pub fn new(sender: Box<dyn MessageSender>) -> Result<Self>;
    pub fn navigate(&mut self, url: &str) -> Result<()>;
    pub fn execute_script(&mut self, script: &str) -> Result<String>;
    pub fn get_dom(&self) -> Result<String>;
}
```

### browser_shell API
```rust
pub struct BrowserShell {
    // Internal implementation
}

impl BrowserShell {
    pub fn new(
        config: ShellConfig,
        sender: Box<dyn MessageSender>,
        runtime: Arc<Runtime>
    ) -> Result<Self>;

    pub fn run(&mut self) -> Result<()>;
    pub fn create_tab(&mut self) -> Result<u32>;
    pub fn close_tab(&mut self, tab_id: u32) -> Result<()>;
    pub fn switch_to_tab(&mut self, tab_id: u32) -> Result<()>;
    pub fn get_tab_count(&self) -> usize;
}
```

### cli_app API
```rust
pub struct BrowserApp {
    // Internal implementation
}

impl BrowserApp {
    pub fn new(config: Config) -> Result<Self>;
    pub fn run(self) -> Result<()>;
}
```

## Technology Stack

### Shared Dependencies (All Components)
- **serde** 1.0 (serialization)
- **anyhow** 1.0 (error handling)
- **thiserror** 1.0 (error types)

### Component-Specific Dependencies

**message_bus:**
- crossbeam-channel 0.5 (IPC)
- tokio 1.35 (async runtime)

**network_stack:**
- reqwest 0.11 (HTTP client)
- url 2.5 (URL parsing)
- cookie_store 0.20 (cookies)

**adblock_engine:**
- adblock 0.8 (filter engine)

**browser_core:**
- rusqlite 0.30 (history/bookmarks database)

**webview_integration:**
- wry 0.35 (webview)
- webkit2gtk 2.0 (Linux)
- webview2-com 0.28 (Windows)
- cocoa 0.25 (macOS)

**browser_shell:**
- tao 0.25 (window management)
- egui 0.24 (UI framework)
- eframe 0.24 (UI framework)

**cli_app:**
- tracing 0.1 (logging)
- tracing-subscriber 0.3 (logging)

## Quality Standards

### All Components Must Meet:
- ✅ Test coverage ≥ 80%
- ✅ TDD compliance (Red-Green-Refactor git history)
- ✅ Zero linting errors (cargo clippy)
- ✅ Formatted code (cargo fmt)
- ✅ Documentation for public APIs
- ✅ No unsafe code without justification
- ✅ Contract compliance verification

### Project-Level Targets:
- ✅ WPT subset 40% pass rate
- ✅ ACID1 test: PASS
- ✅ Top 10 websites load successfully
- ✅ Memory usage < 500MB single tab
- ✅ Ad block effectiveness > 90%

## Development Timeline

**Week 1:** Base + Core components (Waves 1-3)
**Week 2:** Feature components + Integration (Waves 4-5)
**Week 3:** Application + Testing + Polish (Wave 6 + QA)

## Success Criteria

### Phase 1 Complete When:
- ✅ All 9 components pass 11-check verification
- ✅ All unit tests passing (100%)
- ✅ All integration tests passing (100%)
- ✅ Can load google.com successfully
- ✅ Ad blocking functional on major websites
- ✅ Tab management working
- ✅ WPT subset ≥ 40% pass rate
- ✅ ACID1 test passing
- ✅ No crashes during 1-hour usage

## Component Size Tracking

| Component | Estimated Tokens | Estimated LOC | Status |
|-----------|------------------|---------------|--------|
| shared_types | 5,000 | 500 | Planned |
| message_bus | 8,000 | 800 | Planned |
| config_manager | 6,000 | 600 | Planned |
| network_stack | 12,000 | 1,200 | Planned |
| adblock_engine | 8,000 | 800 | Planned |
| browser_core | 10,000 | 1,000 | Planned |
| webview_integration | 12,000 | 1,200 | Planned |
| browser_shell | 14,000 | 1,400 | Planned |
| cli_app | 3,000 | 300 | Planned |
| **TOTAL** | **78,000** | **7,800** | **Planned** |

All components well within optimal limit of 70,000 tokens!
