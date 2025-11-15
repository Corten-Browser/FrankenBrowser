# Web Platform Tests (WPT) Integration

This directory contains the infrastructure for running Web Platform Tests against FrankenBrowser.

## Quick Start

```bash
# Run Phase 1 test subset (recommended for first run)
./run_wpt.sh

# Run all tests (may take hours)
./run_wpt.sh --all

# Run specific category
./run_wpt.sh --category html
```

## Prerequisites

- Python 3.7+
- Git  
- Rust/Cargo (for building WebDriver server)

## Architecture

The test infrastructure automatically:
1. Clones WPT repository (if not present)
2. Builds WebDriver server (if needed)
3. Sets up Python virtual environment
4. Runs tests and analyzes results

## Test Results

Results are saved to:
- `tests/wpt/results/wpt-results-TIMESTAMP.json` - Raw test results
- `tests/wpt/logs/wpt-run-TIMESTAMP.log` - Execution logs

## Phase 1 Target: 40% Pass Rate

Focus areas:
- WebDriver protocol tests (60-70% expected)
- Basic HTML/DOM tests (30-40% expected)
- Navigation tests (50-60% expected)

## Analyzing Results

```bash
python3 tests/wpt/analyze_results.py tests/wpt/results/wpt-results-*.json
```

## Known Limitations

1. GUI tests may fail (Xvfb issue) - test on real X11 display
2. Some endpoints are placeholders (screenshot, element finding)
3. WebDriver runs standalone (not integrated with browser yet)

**Expected initial pass rate: 10-20%**
**Target after integration: 40%+**

## Next Steps

1. Run baseline: `./run_wpt.sh`
2. Analyze results
3. Implement missing features
4. Integrate WebDriver with browser_core
5. Re-test and iterate

See docs/WPT_INTEGRATION.md for detailed integration plan.
