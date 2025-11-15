#!/bin/bash
# Test WebDriver server HTTP endpoints

PORT=4444
BASE_URL="http://127.0.0.1:$PORT"

echo "Testing WebDriver HTTP server..."
echo "================================"

# Test 1: Status endpoint
echo ""
echo "Test 1: GET /status"
curl -s -X GET "$BASE_URL/status" | jq '.' || echo "Failed or no jq"

# Test 2: Create session
echo ""
echo "Test 2: POST /session"
SESSION_RESPONSE=$(curl -s -X POST "$BASE_URL/session" \
  -H "Content-Type: application/json" \
  -d '{
    "capabilities": {
      "alwaysMatch": {
        "browserName": "frankenbrowser",
        "browserVersion": "0.1.0",
        "platformName": "linux"
      }
    }
  }')
echo "$SESSION_RESPONSE" | jq '.' || echo "$SESSION_RESPONSE"

# Extract session ID
SESSION_ID=$(echo "$SESSION_RESPONSE" | jq -r '.value.sessionId' 2>/dev/null)

if [ -z "$SESSION_ID" ] || [ "$SESSION_ID" == "null" ]; then
  echo "Failed to create session"
  exit 1
fi

echo "Session ID: $SESSION_ID"

# Test 3: Navigate
echo ""
echo "Test 3: POST /session/$SESSION_ID/url"
curl -s -X POST "$BASE_URL/session/$SESSION_ID/url" \
  -H "Content-Type: application/json" \
  -d '{"url": "https://www.example.com"}' | jq '.' || echo "OK (no JSON response)"

# Test 4: Get current URL
echo ""
echo "Test 4: GET /session/$SESSION_ID/url"
curl -s -X GET "$BASE_URL/session/$SESSION_ID/url" | jq '.' || echo "Failed"

# Test 5: Get window handle
echo ""
echo "Test 5: GET /session/$SESSION_ID/window"
curl -s -X GET "$BASE_URL/session/$SESSION_ID/window" | jq '.' || echo "Failed"

# Test 6: Delete session
echo ""
echo "Test 6: DELETE /session/$SESSION_ID"
curl -s -X DELETE "$BASE_URL/session/$SESSION_ID" || echo "OK"

echo ""
echo "================================"
echo "WebDriver server tests complete!"
