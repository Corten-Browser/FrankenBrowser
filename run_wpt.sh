#!/bin/bash
# Run Web Platform Tests for FrankenBrowser
#
# Usage:
#   ./run_wpt.sh                    # Run default test subset
#   ./run_wpt.sh --all              # Run all tests
#   ./run_wpt.sh --category html    # Run specific category

set -e

# Configuration
WPT_DIR="../wpt"
ADAPTER_DIR="wpt_adapter"
RESULTS_DIR="tests/wpt/results"
LOGS_DIR="tests/wpt/logs"

# Colors for output
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Create directories
mkdir -p "$RESULTS_DIR" "$LOGS_DIR"

echo "FrankenBrowser Web Platform Tests Runner"
echo "========================================"
echo ""

# Check if WPT is cloned
if [ ! -d "$WPT_DIR" ]; then
    echo -e "${YELLOW}WPT repository not found at $WPT_DIR${NC}"
    echo "Cloning WPT repository..."
    git clone --depth=1 https://github.com/web-platform-tests/wpt.git "$WPT_DIR"
    echo -e "${GREEN}✓ WPT cloned${NC}"
    echo ""
fi

# Build WebDriver server if needed
if [ ! -f "target/release/webdriver-server" ]; then
    echo "Building WebDriver server..."
    cargo build --release --bin webdriver-server
    echo -e "${GREEN}✓ WebDriver server built${NC}"
    echo ""
fi

# Determine test subset
TEST_ARGS=""
if [ "$1" == "--all" ]; then
    echo "Running ALL tests (this may take hours)..."
    TEST_ARGS=""
elif [ "$1" == "--category" ] && [ -n "$2" ]; then
    echo "Running tests for category: $2"
    TEST_ARGS="--include=$2"
else
    echo "Running Phase 1 test subset (basic functionality)..."
    # Create test inclusion file if it doesn't exist
    if [ ! -f "tests/wpt/phase1-tests.txt" ]; then
        cat > tests/wpt/phase1-tests.txt <<EOF
# Phase 1 WPT Tests - Basic functionality
# Target: 40% pass rate

# HTML - Basic document structure
/html/semantics/document-metadata
/html/semantics/scripting-1

# Fetch API - Basic requests
/fetch/api/basic

# DOM - Basic queries
/dom/nodes

# WebDriver - Protocol compliance
/webdriver/tests/sessions
/webdriver/tests/navigation
EOF
    fi
    TEST_ARGS="--include-file=tests/wpt/phase1-tests.txt"
fi

# Set up Python environment for WPT
echo "Setting up WPT environment..."
cd "$WPT_DIR"

# Install WPT dependencies if needed
if [ ! -d "tools/wptrunner/venv" ]; then
    echo "Installing WPT dependencies..."
    pip3 install --user virtualenv 2>/dev/null || true
    python3 -m virtualenv tools/wptrunner/venv
    source tools/wptrunner/venv/bin/activate
    pip install -e tools/wptrunner
else
    source tools/wptrunner/venv/bin/activate
fi

echo -e "${GREEN}✓ WPT environment ready${NC}"
echo ""

# Run tests
echo "Running WPT tests..."
echo "Logs: $LOGS_DIR/wpt-run.log"
echo ""

TIMESTAMP=$(date +%Y%m%d-%H%M%S)
RESULT_FILE="../$RESULTS_DIR/wpt-results-$TIMESTAMP.json"
LOG_FILE="../$LOGS_DIR/wpt-run-$TIMESTAMP.log"

python3 tools/wptrunner/wptrunner \
    --product=webdriver \
    --webdriver-binary=../target/release/webdriver-server \
    --webdriver-port=4444 \
    --log-raw="$RESULT_FILE" \
    --log-wptreport="$RESULT_FILE.txt" \
    $TEST_ARGS \
    2>&1 | tee "$LOG_FILE"

EXIT_CODE=${PIPESTATUS[0]}

cd - > /dev/null

echo ""
echo "========================================"
if [ $EXIT_CODE -eq 0 ]; then
    echo -e "${GREEN}✓ WPT tests completed${NC}"
else
    echo -e "${YELLOW}⚠ WPT tests completed with some failures${NC}"
fi

echo ""
echo "Results saved to: $RESULT_FILE"
echo "Logs saved to: $LOG_FILE"
echo ""

# Analyze results
if [ -f "$RESULT_FILE" ]; then
    echo "Analyzing results..."
    python3 tests/wpt/analyze_results.py "$RESULT_FILE"
fi

exit $EXIT_CODE
