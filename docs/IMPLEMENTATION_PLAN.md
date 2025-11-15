# FrankenBrowser Remaining Implementation Plan
**Created:** 2025-11-15
**Version:** 0.1.0 → 0.2.0 target
**Estimated Total Time:** 40-60 hours
**Parallel Implementation Strategy:** Multi-agent orchestration

## Overview

This plan implements all remaining autonomous requirements from the frankenstein-browser-specification.md that can be completed without user support.

**Current Compliance:** ~75%
**Target Compliance:** ~95%

---

## Phase 0: Specification Analysis ✓

**Status:** Complete
**Requirements Extracted:** 47 items across 8 categories

### Requirement Categories:
- FR (Functional Requirements): 18 items
- NFR (Non-Functional Requirements): 8 items
- WD (WebDriver Protocol): 12 items
- TEST (Test Infrastructure): 5 items
- BUILD (Build System): 4 items

---

## Phase 1: Build Infrastructure (2-3 hours)

**Component:** Root build scripts and resources
**Level:** 0 (Base)
**Dependencies:** None
**Can run in parallel:** Yes

### Tasks:
1. **Create root build.rs**
   - Platform detection (Windows/Linux/macOS)
   - Windows icon compilation (winres)
   - EasyList download automation
   - Platform-specific compiler flags

2. **Create resources/init.js**
   - WebView initialization script
   - IPC message handler registration
   - Console redirection to Rust
   - DOM ready event listener
   - Browser API polyfills

3. **Create icon resources**
   - Generate placeholder browser.ico (Windows)
   - Generate placeholder browser.png (Linux/macOS)
   - Generate placeholder browser.icns (macOS)
   - Use simple SVG→PNG conversion

### Deliverables:
- `/home/user/FrankenBrowser/build.rs`
- `/home/user/FrankenBrowser/resources/init.js`
- `/home/user/FrankenBrowser/resources/icons/browser.{ico,png,icns}`

### Requirements Satisfied:
- BUILD-001: Root build script
- BUILD-002: JavaScript initialization
- BUILD-003: Application icons

---

## Phase 2: Network Stack Components (4-6 hours)

**Component:** network_stack
**Level:** 1 (Core)
**Dependencies:** shared_types, message_bus
**Can run in parallel:** After Phase 1

### Tasks:

#### 2.1 HTTP Cache Implementation (`cache.rs`)
- **In-memory cache** with LRU eviction
- **Disk cache** using rusqlite
- **Cache-Control header** parsing
- **ETag/Last-Modified** support
- **Cache invalidation** logic
- **Size limits** and cleanup
- **Integration** with NetworkStack

**Estimated:** 3-4 hours

#### 2.2 Request Handler (`request_handler.rs`)
- **Request interceptor** interface
- **Pre-request filtering** (ad blocking integration)
- **Request modification** (header injection)
- **Response transformation** (content filtering)
- **Request lifecycle hooks**

**Estimated:** 2-3 hours

### Deliverables:
- `components/network_stack/src/cache.rs` (350-400 lines)
- `components/network_stack/src/request_handler.rs` (250-300 lines)
- Unit tests for cache operations
- Integration tests for request pipeline

### Requirements Satisfied:
- NFR-002: HTTP caching
- NFR-003: Request interception
- FR-008: Cache management

---

## Phase 3: WebView Integration Components (7-10 hours)

**Component:** webview_integration
**Level:** 2 (Feature)
**Dependencies:** shared_types, message_bus, network_stack
**Can run in parallel:** After Phase 2

### Tasks:

#### 3.1 JavaScript Bridge (`javascript_bridge.rs`)
- **IPC message dispatcher**
- **Callback registration** system
- **Event emitter** (Rust → JS)
- **Type serialization** (serde_json)
- **Error propagation** from JS to Rust
- **Script injection** utilities

**Estimated:** 3-4 hours

#### 3.2 Platform-Specific Code (`platform/`)

**3.2.1 Linux/WebKit2GTK (`linux.rs`)**
- WebKit2GTK initialization
- Cookie manager setup
- User agent customization
- JavaScript execution context
- Signal handlers (navigation, load finished)

**Estimated:** 2-3 hours

**3.2.2 Windows/WebView2 (`windows.rs`)**
- WebView2 COM initialization
- Environment setup
- CoreWebView2 configuration
- Script injection timing
- Navigation handlers

**Estimated:** 2-3 hours

**3.2.3 macOS/WKWebView (`macos.rs`)**
- WKWebView initialization
- Configuration setup
- User script injection
- Navigation delegates
- JavaScript message handlers

**Estimated:** 2-3 hours (can be placeholder/stub)

### Deliverables:
- `components/webview_integration/src/javascript_bridge.rs` (400-500 lines)
- `components/webview_integration/src/platform/mod.rs` (100 lines)
- `components/webview_integration/src/platform/linux.rs` (300-400 lines)
- `components/webview_integration/src/platform/windows.rs` (300-400 lines)
- `components/webview_integration/src/platform/macos.rs` (300-400 lines, stub)
- Unit tests for IPC bridge
- Platform-specific integration tests

### Requirements Satisfied:
- FR-009: JavaScript bridge
- FR-010: Platform abstraction
- NFR-004: Cross-platform support

---

## Phase 4: Browser Shell UI Components (6-8 hours)

**Component:** browser_shell
**Level:** 2 (Feature)
**Dependencies:** shared_types, message_bus, webview_integration
**Can run in parallel:** After Phase 3

### Tasks:

#### 4.1 UI Components (`ui_components.rs`)
- **URLBar widget** (input field with suggestions)
- **NavigationButtons** (back, forward, reload, home)
- **TabBar** (tab labels, close buttons, new tab)
- **StatusBar** (loading indicator, security status)
- **Toolbar layout** management
- **Event handling** (button clicks, URL entry)

**Estimated:** 4-5 hours

#### 4.2 Menu System (`menu.rs`)
- **File Menu**: New Tab, New Window, Close, Quit
- **Edit Menu**: Copy, Paste, Find, Preferences
- **View Menu**: Zoom In/Out/Reset, Full Screen, Developer Tools
- **History Menu**: Back, Forward, Show All History
- **Bookmarks Menu**: Add Bookmark, Show All Bookmarks
- **Help Menu**: About FrankenBrowser
- **Menu bar integration** with tao/wry
- **Keyboard shortcuts** (Ctrl+T, Ctrl+W, etc.)

**Estimated:** 3-4 hours

### Deliverables:
- `components/browser_shell/src/ui_components.rs` (600-700 lines)
- `components/browser_shell/src/menu.rs` (400-500 lines)
- UI component tests (snapshot/visual testing if possible)
- Integration tests for menu actions

### Requirements Satisfied:
- FR-011: URL bar
- FR-012: Navigation controls
- FR-013: Tab management UI
- FR-014: Menu system

---

## Phase 5: WebDriver Protocol Enhancement (10-14 hours)

**Component:** webdriver
**Level:** 3 (Integration)
**Dependencies:** browser_core, webview_integration, browser_shell
**Can run in parallel:** After Phase 4

### Tasks:

#### 5.1 Element Finding & Interaction
**File:** `components/webdriver/src/server.rs` (enhance existing)

**Element Finding:**
- Integrate with WebView DOM (via JavaScript bridge)
- Support all locator strategies:
  - CSS Selector
  - XPath
  - ID
  - Name
  - Tag Name
  - Class Name
  - Link Text
  - Partial Link Text
- findElement (single)
- findElements (multiple)
- Element caching with UUIDs

**Estimated:** 4-5 hours

**Element Interaction:**
- Click element (POST /element/:id/click)
- Send keys (POST /element/:id/value)
- Clear input (POST /element/:id/clear)
- Get element text (GET /element/:id/text)
- Get element attribute (GET /element/:id/attribute/:name)
- Get element property (GET /element/:id/property/:name)
- Get element CSS value (GET /element/:id/css/:propertyName)
- Is element displayed (GET /element/:id/displayed)
- Is element enabled (GET /element/:id/enabled)
- Is element selected (GET /element/:id/selected)

**Estimated:** 4-5 hours

#### 5.2 Window Management
- getAllWindowHandles
- switchToWindow
- closeWindow
- getWindowRect
- setWindowRect
- maximizeWindow
- minimizeWindow

**Estimated:** 2-3 hours

#### 5.3 Advanced Script Execution
- executeAsyncScript with callback
- Argument serialization (primitive types, elements)
- Return value deserialization
- Timeout handling

**Estimated:** 2-3 hours

### Deliverables:
- Enhanced `components/webdriver/src/server.rs` (+800-1000 lines)
- New `components/webdriver/src/element.rs` (400-500 lines)
- New `components/webdriver/src/dom_interface.rs` (300-400 lines)
- Comprehensive WebDriver endpoint tests
- W3C WebDriver spec compliance tests

### Requirements Satisfied:
- WD-001 to WD-012: All WebDriver endpoints
- TEST-001: WebDriver automation support

---

## Phase 6: Browser Core Navigation (3-4 hours)

**Component:** browser_core
**Level:** 2 (Feature)
**Dependencies:** shared_types, network_stack
**Can run in parallel:** With Phase 2

### Tasks:

#### 6.1 Navigation Module (`navigation.rs`)
- **URL validation** (protocol, domain, port)
- **Protocol handling**:
  - http/https (via NetworkStack)
  - file:// (local file loading)
  - data: (data URLs)
  - about: (internal pages)
- **Redirect following** with cycle detection
- **Error page generation** (404, timeout, SSL error)
- **Navigation state machine** (loading, loaded, error)
- **Cancel navigation** support

**Estimated:** 3-4 hours

### Deliverables:
- `components/browser_core/src/navigation.rs` (400-500 lines)
- Navigation state tests
- Protocol handler tests
- Error page rendering tests

### Requirements Satisfied:
- FR-003: URL navigation
- FR-004: Protocol support
- NFR-005: Error handling

---

## Phase 7: Test Infrastructure (10-12 hours)

**Component:** Test harnesses and benchmarks
**Level:** 4 (Testing)
**Dependencies:** All components
**Can run in parallel:** After Phases 1-6

### Tasks:

#### 7.1 WPT Test Harness (`tests/wpt/`)

**7.1.1 Test Runner Implementation**
- Clone WPT repository automation
- Test execution engine
- WebDriver integration
- Result collection and parsing
- Pass/fail determination
- Timeout handling
- Test filtering (Phase 1 subset)

**Estimated:** 4-5 hours

**7.1.2 Result Reporting**
- JSON result output
- HTML report generation
- Pass rate calculation
- Regression detection
- Database storage (rusqlite)

**Estimated:** 2-3 hours

#### 7.2 Performance Benchmarks (`benchmarks/`)

**7.2.1 Page Load Benchmark**
- Measure time to first byte
- Measure time to DOM ready
- Measure time to fully loaded
- Network waterfall analysis
- Compare against baseline

**Estimated:** 1-2 hours

**7.2.2 Tab Switching Benchmark**
- Measure tab switch latency
- Memory usage per tab
- CPU usage during switch

**Estimated:** 1 hour

**7.2.3 Memory Usage Benchmark**
- Track memory over time
- Detect memory leaks
- Compare with baseline

**Estimated:** 1-2 hours

**7.2.4 Ad Blocking Overhead**
- Measure blocking decision time
- Count blocked resources
- Compare with/without blocking

**Estimated:** 1 hour

#### 7.3 Test Automation Scripts
- `scripts/run-wpt.sh` - WPT execution
- `scripts/check-performance.sh` - Benchmark runner
- `tests/phase1-tests.txt` - WPT test list
- `tests/wpt-metadata/` - Test expectations

**Estimated:** 1-2 hours

### Deliverables:
- `tests/wpt/harness.rs` (complete implementation, +400 lines)
- `tests/wpt/runner.rs` (new, 500-600 lines)
- `tests/wpt/reporter.rs` (new, 300-400 lines)
- `benchmarks/page_load.rs` (200-250 lines)
- `benchmarks/tab_switching.rs` (150-200 lines)
- `benchmarks/memory_usage.rs` (200-250 lines)
- `benchmarks/adblock_overhead.rs` (150-200 lines)
- `scripts/run-wpt.sh` (100-150 lines bash)
- `scripts/check-performance.sh` (80-100 lines bash)
- `tests/phase1-tests.txt` (test list)

### Requirements Satisfied:
- TEST-002: WPT integration
- TEST-003: Performance benchmarks
- TEST-004: Automated testing
- NFR-006: Performance monitoring

---

## Phase 8: Integration Testing (4-6 hours)

**Component:** System integration
**Level:** System
**Dependencies:** All components

### Tasks:
1. **Cross-component integration tests**
   - Browser startup with all components
   - Navigate via WebDriver
   - Execute JavaScript via bridge
   - Cache behavior verification
   - UI interaction tests

2. **End-to-end workflows**
   - Complete page load cycle
   - Form submission
   - Multi-tab scenarios
   - Bookmark management
   - History navigation

3. **Contract compliance**
   - Verify all message bus contracts
   - Verify WebDriver protocol compliance
   - Verify W3C spec conformance

**Estimated:** 4-6 hours

### Deliverables:
- `tests/integration/full_browser_test.rs` (new, 400-500 lines)
- `tests/integration/webdriver_integration.rs` (new, 300-400 lines)
- `tests/integration/ui_workflow_test.rs` (new, 300-400 lines)
- Updated `tests/end_to_end_test.rs` (enhanced)

### Requirements Satisfied:
- Integration test coverage
- System validation
- Contract verification

---

## Phase 9: Quality Verification (2-3 hours)

**Component:** All components
**Level:** Validation

### Tasks:
1. **Run 12-check verification** on each modified component:
   - ✅ Tests Pass (100%)
   - ✅ Imports Resolve
   - ✅ No Stubs
   - ✅ No TODOs
   - ✅ Documentation Complete
   - ✅ No Remaining Work Markers
   - ✅ Test Coverage ≥80%
   - ✅ Manifest Complete
   - ✅ Defensive Programming
   - ✅ Semantic Correctness
   - ✅ Contract Compliance
   - ✅ Test Quality

2. **Fix any issues** identified by verification

3. **Generate quality reports**
   - Component quality scores
   - Overall project quality
   - Test coverage dashboard

4. **System validation**
   - Requirements traceability
   - Specification compliance percentage
   - Deployment readiness assessment

**Estimated:** 2-3 hours

---

## Implementation Strategy

### Parallel Execution Plan

Following orchestration max_parallel_agents = 3:

**Wave 1 (Parallel - Level 0):**
1. Agent: Build Infrastructure (Phase 1)
2. Agent: Network Cache (Phase 2.1)
3. Agent: Navigation Module (Phase 6)

**Wave 2 (Parallel - Level 1):**
1. Agent: Request Handler (Phase 2.2)
2. Agent: JavaScript Bridge (Phase 3.1)
3. Agent: Platform Code - Linux (Phase 3.2.1)

**Wave 3 (Parallel - Level 2):**
1. Agent: Platform Code - Windows (Phase 3.2.2)
2. Agent: UI Components (Phase 4.1)
3. Agent: Menu System (Phase 4.2)

**Wave 4 (Parallel - Level 3):**
1. Agent: WebDriver Element Finding (Phase 5.1)
2. Agent: WebDriver Window Management (Phase 5.2)
3. Agent: WPT Harness (Phase 7.1)

**Wave 5 (Parallel - Level 3):**
1. Agent: WebDriver Element Interaction (Phase 5.1)
2. Agent: WebDriver Script Execution (Phase 5.3)
3. Agent: Performance Benchmarks (Phase 7.2)

**Wave 6 (Sequential - Integration):**
1. Integration Testing (Phase 8)
2. Quality Verification (Phase 9)

### Total Estimated Time
- **Sequential:** 54-74 hours
- **With 3-agent parallelism:** 22-30 hours

---

## Success Criteria

### Phase Completion Gates:
- ✅ All component tests pass (100%)
- ✅ Integration tests pass (100%)
- ✅ Test coverage ≥80% for new code
- ✅ Zero clippy warnings
- ✅ All 12 quality checks pass
- ✅ WPT harness functional (tests run successfully)
- ✅ Benchmarks baseline established

### Overall Success:
- **Specification Compliance:** ≥95%
- **Test Pass Rate:** 100% (all test types)
- **Code Quality Score:** ≥90/100
- **WebDriver Compliance:** W3C spec conformant
- **WPT Infrastructure:** Ready for test execution
- **Build System:** Cross-platform support

---

## Files to Create/Modify

### New Files (29 files):
1. `build.rs`
2. `resources/init.js`
3. `resources/icons/browser.ico`
4. `resources/icons/browser.png`
5. `resources/icons/browser.icns`
6. `components/network_stack/src/cache.rs`
7. `components/network_stack/src/request_handler.rs`
8. `components/webview_integration/src/javascript_bridge.rs`
9. `components/webview_integration/src/platform/mod.rs`
10. `components/webview_integration/src/platform/linux.rs`
11. `components/webview_integration/src/platform/windows.rs`
12. `components/webview_integration/src/platform/macos.rs`
13. `components/browser_shell/src/ui_components.rs`
14. `components/browser_shell/src/menu.rs`
15. `components/browser_core/src/navigation.rs`
16. `components/webdriver/src/element.rs`
17. `components/webdriver/src/dom_interface.rs`
18. `tests/wpt/runner.rs`
19. `tests/wpt/reporter.rs`
20. `benchmarks/page_load.rs`
21. `benchmarks/tab_switching.rs`
22. `benchmarks/memory_usage.rs`
23. `benchmarks/adblock_overhead.rs`
24. `scripts/run-wpt.sh`
25. `scripts/check-performance.sh`
26. `tests/phase1-tests.txt`
27. `tests/integration/full_browser_test.rs`
28. `tests/integration/webdriver_integration.rs`
29. `tests/integration/ui_workflow_test.rs`

### Modified Files (8 files):
1. `components/network_stack/src/lib.rs` (add cache module)
2. `components/network_stack/src/types.rs` (add cache types)
3. `components/webview_integration/src/lib.rs` (add bridge module)
4. `components/webview_integration/src/types.rs` (add bridge types)
5. `components/browser_shell/src/lib.rs` (add UI modules)
6. `components/browser_core/src/lib.rs` (add navigation module)
7. `components/webdriver/src/server.rs` (enhance endpoints)
8. `tests/wpt/harness.rs` (complete implementation)

### Total Lines of Code to Add: ~8,500-10,500 lines

---

## Risk Mitigation

### Known Risks:
1. **GUI Testing Block:** Xvfb compatibility issue
   - **Mitigation:** Use headless mode for automation, document GUI limitation

2. **Platform-Specific Code:** macOS/Windows untested
   - **Mitigation:** Create stubs, mark as experimental, test on CI when available

3. **WPT Pass Rate:** May not reach 40% without GUI
   - **Mitigation:** Focus on headless-testable WPT tests, document limitations

### Fallback Plans:
- If GUI tests fail: Mark as `#[ignore]` and document requirement
- If platform code fails to compile: Use conditional compilation `#[cfg(target_os)]`
- If WPT integration fails: Ensure harness infrastructure is complete, tests can be run manually

---

## Post-Implementation

### Documentation Updates:
- Update README.md with new features
- Update specification compliance percentage
- Document new WebDriver endpoints
- Update architecture diagrams
- Create performance baseline document

### Version Bump:
- Current: 0.1.0
- Target: 0.2.0 (minor version bump for new features)
- Lifecycle: pre-release (no major version change without approval)

### Next Steps (Beyond This Plan):
- Fix Xvfb compatibility (requires environment debugging)
- Run WPT tests against live browser (requires GUI)
- Optimize performance based on benchmarks
- Add extension system (future feature)
- Implement developer tools integration (future feature)

---

**Plan Status:** Ready for Implementation
**Orchestrator:** Ready to launch multi-agent parallel development
**Estimated Completion:** 22-30 hours (with 3-agent parallelism)
