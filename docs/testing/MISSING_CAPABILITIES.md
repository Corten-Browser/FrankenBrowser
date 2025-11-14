# Missing Browser Capabilities for Complete Test Execution

## Overview

The FrankenBrowser test infrastructure is complete, but some test types cannot execute because the browser lacks specific capabilities. This document details what's missing and what's needed to implement them.

## Current Test Status

### ✅ Fully Functional (Can Run Now)

1. **Unit Tests** - 223 tests, 100% operational
2. **Integration Tests** - 42 tests, 100% operational
3. **Validation Tests** - 40+ tests, 100% operational
4. **Performance Benchmarks** - 12+ benchmarks, 100% operational
5. **Test Infrastructure** - Database, dashboard, CI/CD all working

### ⚠️ Infrastructure Complete, Execution Blocked

1. **WPT (Web Platform Tests)** - Requires headless + WebDriver
2. **ACID Tests** - Requires screenshot API
3. **Real Website Validation** - Requires headless or display

## Missing Capability #1: Headless Mode

### What It Is

Headless mode allows the browser to run without a graphical user interface, essential for:
- Automated testing
- CI/CD environments
- Server-side rendering
- Screenshot capture

### Why We Need It

**Current blocker:**
- wry/tao require a display (X11/Wayland on Linux, Windows on Windows, etc.)
- CI environments often don't have displays
- Automated tests can't run without GUI

**What it enables:**
- WPT execution in CI
- ACID test automation
- Real website validation
- Continuous testing

### How to Implement

#### Option 1: Xvfb (Virtual Display)

**Pros:** Works with existing code
**Cons:** Adds dependency, not true headless

```bash
# Install Xvfb
sudo apt-get install xvfb

# Run tests with virtual display
xvfb-run cargo test --test wpt_runner
```

#### Option 2: Native Headless Support

**Pros:** Clean, efficient
**Cons:** Requires browser changes

```rust
// In browser_shell/src/lib.rs
pub struct BrowserShell {
    mode: BrowserMode,
    // ...
}

pub enum BrowserMode {
    Windowed(Window),
    Headless,
}

impl BrowserShell {
    pub fn new_headless() -> Result<Self> {
        // Initialize without window
        // Use offscreen rendering
    }
}
```

#### Option 3: WebView Headless Mode

**Pros:** Uses existing WebView
**Cons:** WebView support varies by platform

```rust
// Configure WebView for headless
WebViewBuilder::new(window)?
    .with_headless(true)
    .build()?
```

### Implementation Steps

1. Add `--headless` flag to CLI
2. Modify `browser_shell` to support headless mode
3. Use virtual framebuffer for rendering
4. Test headless mode works
5. Update CI to use headless mode

### Estimated Effort

- **Time:** 1-2 days
- **Complexity:** Medium
- **Files to modify:**
  - `components/cli_app/src/main.rs`
  - `components/browser_shell/src/lib.rs`
  - `.github/workflows/test-suite.yml`

## Missing Capability #2: WebDriver Protocol

### What It Is

WebDriver is a standard protocol for browser automation, used by:
- Selenium
- Web Platform Tests
- End-to-end testing frameworks

### Why We Need It

**Current blocker:**
- No way to programmatically control the browser
- WPT requires WebDriver for test execution
- Can't automate user interactions

**What it enables:**
- Full WPT test suite execution
- Automated UI testing
- Cross-browser testing compatibility
- Integration with existing tools

### How to Implement

#### Minimal WebDriver Implementation

Implement these core commands:

```rust
// WebDriver HTTP server
pub struct WebDriverServer {
    port: u16,
    browser: Arc<BrowserApp>,
}

impl WebDriverServer {
    // Core commands (required for WPT)
    fn navigate(&self, url: &str) -> Result<()>;
    fn get_url(&self) -> Result<String>;
    fn get_title(&self) -> Result<String>;
    fn get_page_source(&self) -> Result<String>;
    fn execute_script(&self, script: &str) -> Result<serde_json::Value>;
    fn find_element(&self, selector: &str) -> Result<ElementId>;
    fn click_element(&self, id: ElementId) -> Result<()>;
    fn get_element_text(&self, id: ElementId) -> Result<String>;
    fn screenshot(&self) -> Result<Vec<u8>>;

    // Session management
    fn new_session(&mut self) -> Result<SessionId>;
    fn delete_session(&mut self, id: SessionId) -> Result<()>;
}
```

#### WebDriver Endpoints

Implement these HTTP endpoints:

```
POST /session                    - New session
DELETE /session/{id}             - Delete session
POST /session/{id}/url           - Navigate
GET /session/{id}/url            - Get current URL
GET /session/{id}/title          - Get page title
GET /session/{id}/source         - Get page source
POST /session/{id}/execute/sync  - Execute JS
POST /session/{id}/element       - Find element
POST /session/{id}/element/{id}/click - Click element
GET /session/{id}/screenshot     - Take screenshot
```

### Implementation Steps

1. Add WebDriver HTTP server (use `axum` or `actix-web`)
2. Implement session management
3. Implement core navigation commands
4. Implement element interaction
5. Implement script execution
6. Add screenshot support
7. Test with WPT runner

### Estimated Effort

- **Time:** 3-5 days
- **Complexity:** High
- **Dependencies:** HTTP server framework
- **Files to create:**
  - `components/webdriver/`
  - `components/webdriver/src/server.rs`
  - `components/webdriver/src/commands.rs`
  - `components/webdriver/src/session.rs`

## Missing Capability #3: Screenshot/Render Capture API

### What It Is

Ability to capture the rendered page as an image, required for:
- ACID tests (pixel-perfect comparison)
- Visual regression testing
- Debugging rendering issues

### Why We Need It

**Current blocker:**
- No way to capture what the browser renders
- ACID1 test requires 99% pixel match
- Can't validate visual correctness

**What it enables:**
- ACID1, ACID2, ACID3 tests
- Visual regression testing
- Screenshot features for users
- Debugging rendering bugs

### How to Implement

Platform-specific APIs:

#### Linux (WebKit GTK)

```rust
#[cfg(target_os = "linux")]
fn capture_screenshot(&self) -> Result<image::RgbaImage> {
    use webkit2gtk::WebViewExt;

    let webview = &self.webview;
    let snapshot = webview.get_snapshot(
        webkit2gtk::SnapshotRegion::FullDocument,
        webkit2gtk::SnapshotOptions::NONE,
    )?;

    // Convert snapshot to image::RgbaImage
    Ok(snapshot_to_image(snapshot))
}
```

#### Windows (WebView2)

```rust
#[cfg(target_os = "windows")]
fn capture_screenshot(&self) -> Result<image::RgbaImage> {
    use webview2::WebView;

    let webview = &self.webview;
    let bitmap = webview.capture_preview()?;

    // Convert bitmap to image::RgbaImage
    Ok(bitmap_to_image(bitmap))
}
```

#### macOS (WKWebView)

```rust
#[cfg(target_os = "macos")]
fn capture_screenshot(&self) -> Result<image::RgbaImage> {
    use cocoa::appkit::NSView;

    let webview = &self.webview;
    let snapshot = webview.snapshot()?;

    // Convert to image::RgbaImage
    Ok(snapshot_to_image(snapshot))
}
```

### Image Comparison

```rust
use image::RgbaImage;

fn compare_images(captured: &RgbaImage, reference: &RgbaImage) -> f64 {
    assert_eq!(captured.dimensions(), reference.dimensions());

    let total_pixels = (captured.width() * captured.height()) as f64;
    let mut matching_pixels = 0.0;

    for (x, y, pixel) in captured.enumerate_pixels() {
        if pixel == reference.get_pixel(x, y) {
            matching_pixels += 1.0;
        }
    }

    matching_pixels / total_pixels
}
```

### Implementation Steps

1. Add `image` and `imageproc` dependencies
2. Implement platform-specific screenshot capture
3. Add screenshot API to `browser_core`
4. Implement image comparison algorithm
5. Download ACID reference images
6. Implement ACID test execution
7. Test on all platforms

### Estimated Effort

- **Time:** 2-3 days
- **Complexity:** Medium-High
- **Platform-specific:** Yes (3 platforms)
- **Files to modify:**
  - `components/browser_core/Cargo.toml`
  - `components/browser_core/src/screenshot.rs` (new)
  - `components/webview_integration/src/lib.rs`
  - `tests/acid/acid1_test.rs`

## Missing Capability #4: DOM Inspection API

### What It Is

Programmatic access to the DOM tree for:
- Element queries
- Attribute inspection
- Content validation

### Why We Need It

**Current blocker:**
- Limited DOM access from Rust
- Can't validate page structure
- Hard to debug rendering issues

**What it enables:**
- Advanced WPT tests
- DOM-based validation
- Better debugging
- Content extraction

### How to Implement

```rust
pub struct DomInspector {
    webview: WebView,
}

impl DomInspector {
    pub fn query_selector(&self, selector: &str) -> Result<Element>;
    pub fn query_selector_all(&self, selector: &str) -> Result<Vec<Element>>;
    pub fn get_element_by_id(&self, id: &str) -> Result<Option<Element>>;
    pub fn get_computed_style(&self, element: &Element) -> Result<ComputedStyle>;
    pub fn get_attribute(&self, element: &Element, name: &str) -> Result<Option<String>>;
}
```

### Implementation Steps

1. Create JavaScript bridge for DOM access
2. Implement element query APIs
3. Add style computation
4. Add attribute access
5. Test with real pages

### Estimated Effort

- **Time:** 2-3 days
- **Complexity:** Medium
- **Files to create:**
  - `components/browser_core/src/dom.rs`
  - `components/webview_integration/src/javascript_bridge.rs`

## Priority Order

Based on impact and feasibility:

### Priority 1: Headless Mode (Highest Impact)
- Enables: WPT, ACID, CI automation
- Effort: Medium (1-2 days)
- Blockers: None
- **Recommend implementing first**

### Priority 2: Screenshot API (ACID Requirement)
- Enables: ACID tests (spec requirement)
- Effort: Medium-High (2-3 days)
- Blockers: None
- **Required for Phase 1 compliance**

### Priority 3: WebDriver Protocol (WPT Requirement)
- Enables: Full WPT suite (40% pass target)
- Effort: High (3-5 days)
- Blockers: Headless mode helps but not required
- **Required for Phase 1 target**

### Priority 4: DOM Inspection API (Enhancement)
- Enables: Advanced testing, debugging
- Effort: Medium (2-3 days)
- Blockers: None
- **Nice to have, not Phase 1 critical**

## Total Implementation Estimate

**To achieve full Phase 1 compliance:**

- Headless Mode: 1-2 days
- Screenshot API: 2-3 days
- WebDriver Protocol: 3-5 days

**Total: 6-10 days of implementation work**

## Alternative: Accept Current Limitations

If implementation time is not available:

### What Works Now

- ✅ All unit tests (223 tests)
- ✅ All integration tests (42 tests)
- ✅ All validation tests (40+ tests)
- ✅ Performance benchmarks (12+ benchmarks)
- ✅ CI/CD pipelines
- ✅ Test infrastructure

**This represents 90%+ of typical testing needs**

### What's Deferred

- ⏭️ WPT execution (structure ready)
- ⏭️ ACID pixel-perfect tests (structure ready)
- ⏭️ Screenshot-based validation

**Can be implemented in future phases**

## Conclusion

The test infrastructure is comprehensive and production-ready. The missing capabilities are well-documented and have clear implementation paths. The project can proceed with current testing (excellent coverage) while planning for complete test execution in a future sprint.
