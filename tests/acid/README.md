# ACID Tests for FrankenBrowser

## Overview

ACID tests are standard rendering compliance tests for web browsers.

**Specification Requirement:** ACID1 must PASS in Phase 1

## ACID Test Levels

### ACID1 ✅ Required for Phase 1
- Tests: CSS1 box model compliance
- Requirement: Must pass with >99% pixel accuracy
- Status: 🚧 Infrastructure complete, execution requires screenshot API

### ACID2 ⏭️ Deferred to Phase 2
- Tests: CSS2.1 compliance
- Status: Not required for Phase 1

### ACID3 ⏭️ Deferred to Phase 3
- Tests: DOM, JavaScript, CSS3
- Status: Not required for Phase 1

## What's Here

- `acid1_test.rs` - ACID1 test implementation
- `fixtures/` - Test files and reference images (to be added)
- `README.md` - This file

## Running ACID1 Test (Once Implemented)

```bash
cargo test --test acid1
```

## What's Needed

### Prerequisites

1. **Download ACID1 Test File**
   ```bash
   mkdir -p tests/acid/fixtures
   cd tests/acid/fixtures
   wget http://www.webstandards.org/files/acid1/test.html -O acid1.html
   wget http://www.webstandards.org/files/acid1/reference.png -O acid1_reference.png
   ```

### Required Browser Capabilities

1. **Screenshot API**
   - Platform-specific render capture
   - Save as PNG format
   - Exact pixel dimensions

2. **Image Comparison**
   - Add `image` crate for loading PNGs
   - Add comparison algorithm (pixelmatch or similar)
   - Calculate similarity score

3. **Headless Mode**
   - Run without GUI
   - Consistent rendering across runs

## Implementation Plan

### Step 1: Add Dependencies
```toml
[dev-dependencies]
image = "0.24"
imageproc = "0.23"
```

### Step 2: Implement Screenshot API

Add to `browser_core`:
```rust
pub fn capture_screenshot(&self) -> Result<image::RgbaImage> {
    // Platform-specific implementation
    #[cfg(target_os = "linux")]
    {
        // Use WebKit GTK screenshot API
    }
    #[cfg(target_os = "windows")]
    {
        // Use WebView2 screenshot API
    }
    #[cfg(target_os = "macos")]
    {
        // Use WKWebView screenshot API
    }
}
```

### Step 3: Implement Image Comparison

```rust
fn compare_images(captured: &RgbaImage, reference: &RgbaImage) -> f64 {
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

### Step 4: Run Test

```rust
let browser = BrowserApp::new(config)?;
browser.navigate("file:///tests/acid/fixtures/acid1.html")?;
browser.wait_for_load(Duration::from_secs(5))?;

let screenshot = browser.capture_screenshot()?;
let reference = image::open("tests/acid/fixtures/acid1_reference.png")?;

let similarity = compare_images(&screenshot, &reference.to_rgba8());
assert!(similarity > 0.99, "ACID1 test failed: {:.2}% similarity", similarity * 100.0);
```

## Current Limitations

⚠️ **Cannot Execute Test Yet**

The test infrastructure is complete but cannot run actual ACID1 test because:

1. No screenshot API - cannot capture rendered page
2. No image comparison - cannot compare to reference
3. No pixel-perfect control - WebView may have variations

## Testing Infrastructure Only

You can test the ACID1 infrastructure:

```bash
cargo test --lib acid1_test
```

This tests:
- Configuration structure
- Placeholder results
- Test requirements

## Pass Criteria

ACID1 passes if:
- Screenshot matches reference image
- Similarity score > 99%
- Visual inspection confirms correct rendering

The test validates:
- ✅ CSS1 box model
- ✅ Basic positioning
- ✅ Font rendering
- ✅ Color handling
- ✅ Border rendering

## References

- ACID1 Test: http://www.webstandards.org/action/acid1/
- Original Specification: http://www.w3.org/Style/CSS/Test/CSS1/current/
- Browser Compatibility: https://en.wikipedia.org/wiki/Acid1

## Next Steps

1. **Implement Screenshot API** in browser_core
2. **Add Image Comparison** using image crate
3. **Download Test Files** to fixtures/
4. **Run and Validate** ACID1 compliance

See `docs/testing/MISSING_CAPABILITIES.md` for detailed requirements.
