#!/bin/bash
# Test newly implemented WebDriver endpoints

PORT=4444
BASE_URL="http://127.0.0.1:$PORT"

echo "Starting WebDriver server..."
cargo run --bin webdriver-server > /tmp/webdriver_test.log 2>&1 &
SERVER_PID=$!
sleep 3

echo "Testing newly implemented WebDriver endpoints"
echo "=============================================="
echo ""

# Create session
echo "Creating session..."
SESSION=$(curl -s -X POST "$BASE_URL/session" \
  -H "Content-Type: application/json" \
  -d '{"capabilities": {}}')
SESSION_ID=$(echo "$SESSION" | jq -r '.value.sessionId')

if [ -z "$SESSION_ID" ] || [ "$SESSION_ID" == "null" ]; then
  echo "Failed to create session"
  kill $SERVER_PID
  exit 1
fi

echo "Session ID: $SESSION_ID"
echo ""

# Test 1: Element finding
echo "1. Element Finding (css selector):"
ELEMENT=$(curl -s -X POST "$BASE_URL/session/$SESSION_ID/element" \
  -H "Content-Type: application/json" \
  -d '{"using": "css selector", "value": "body"}')
echo "$ELEMENT" | jq '.'
echo ""

# Test 2: Element finding with xpath
echo "2. Element Finding (xpath):"
ELEMENT_XPATH=$(curl -s -X POST "$BASE_URL/session/$SESSION_ID/element" \
  -H "Content-Type: application/json" \
  -d '{"using": "xpath", "value": "//div"}')
echo "$ELEMENT_XPATH" | jq '.'
echo ""

# Test 3: Script execution
echo "3. Script Execution:"
SCRIPT=$(curl -s -X POST "$BASE_URL/session/$SESSION_ID/execute/sync" \
  -H "Content-Type: application/json" \
  -d '{"script": "return 2 + 2;", "args": []}')
echo "$SCRIPT" | jq '.'
echo ""

# Test 4: Screenshot
echo "4. Screenshot:"
SCREENSHOT=$(curl -s "$BASE_URL/session/$SESSION_ID/screenshot")
B64=$(echo "$SCREENSHOT" | jq -r '.value')
echo "Base64 length: ${#B64}"
echo "First 100 chars: ${B64:0:100}"
echo ""

# Verify PNG
echo "Verifying PNG signature:"
echo "$B64" | base64 -d | hexdump -C | head -2
echo ""

# Test 5: Invalid locator strategy (should error)
echo "5. Invalid Locator Strategy (should error):"
INVALID=$(curl -s -X POST "$BASE_URL/session/$SESSION_ID/element" \
  -H "Content-Type: application/json" \
  -d '{"using": "invalid", "value": "test"}')
echo "$INVALID" | jq '.'
echo ""

# Clean up
curl -s -X DELETE "$BASE_URL/session/$SESSION_ID"
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null

echo "=============================================="
echo "Test complete!"
