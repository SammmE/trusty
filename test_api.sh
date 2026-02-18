#!/bin/bash

# Test script for Trusty encrypted file storage

API_BASE="http://localhost:3000"
TEST_USERNAME="testuser_$(date +%s)"
TEST_PASSWORD="testpass123"

echo "=== Testing Trusty API ==="
echo ""

# Test 1: Sign Up
echo "1. Testing signup..."
SIGNUP_RESPONSE=$(curl -s -X POST "$API_BASE/api/auth/signup" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$TEST_USERNAME\",\"password\":\"$TEST_PASSWORD\"}")

TOKEN=$(echo $SIGNUP_RESPONSE | grep -o '"access_token":"[^"]*' | cut -d'"' -f4)

if [ -z "$TOKEN" ]; then
  echo "❌ Signup failed"
  echo $SIGNUP_RESPONSE
  exit 1
else
  echo "✅ Signup successful"
  echo "   Token: ${TOKEN:0:20}..."
fi

# Test 2: Login
echo ""
echo "2. Testing login..."
LOGIN_RESPONSE=$(curl -s -X POST "$API_BASE/api/auth/login" \
  -H "Content-Type: application/json" \
  -d "{\"username\":\"$TEST_USERNAME\",\"password\":\"$TEST_PASSWORD\"}")

LOGIN_TOKEN=$(echo $LOGIN_RESPONSE | grep -o '"access_token":"[^"]*' | cut -d'"' -f4)

if [ -z "$LOGIN_TOKEN" ]; then
  echo "❌ Login failed"
  echo $LOGIN_RESPONSE
  exit 1
else
  echo "✅ Login successful"
fi

# Test 3: Get current user
echo ""
echo "3. Testing /api/auth/me..."
ME_RESPONSE=$(curl -s -X GET "$API_BASE/api/auth/me" \
  -H "Authorization: Bearer $TOKEN")

if echo $ME_RESPONSE | grep -q "user_id"; then
  echo "✅ Auth check successful"
  echo "   User: $(echo $ME_RESPONSE | grep -o '"username":"[^"]*' | cut -d'"' -f4)"
else
  echo "❌ Auth check failed"
  echo $ME_RESPONSE
  exit 1
fi

# Test 4: List files (should be empty)
echo ""
echo "4. Testing file list (should be empty)..."
FILES_RESPONSE=$(curl -s -X GET "$API_BASE/api/files" \
  -H "Authorization: Bearer $TOKEN")

if echo $FILES_RESPONSE | grep -q "^\[\]$"; then
  echo "✅ File list is empty (as expected)"
else
  echo "⚠️  File list response: $FILES_RESPONSE"
fi

# Test 5: Create test file and upload
echo ""
echo "5. Testing file upload..."
TEST_FILE="test_file_$(date +%s).txt"
echo "This is a test file created at $(date)" > /tmp/$TEST_FILE

# Create multipart form data manually
BOUNDARY="----WebKitFormBoundary$(date +%s)"
METADATA='{"original_name":"'$TEST_FILE'","mime_type":"text/plain","size_bytes":100,"client_encryption_algo":"AES-GCM-256"}'

# Note: For a real encrypted upload, the file would need to be encrypted first
# This is a simplified test showing the API structure
UPLOAD_RESPONSE=$(curl -s -X POST "$API_BASE/api/files/upload" \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/tmp/$TEST_FILE" \
  -F "metadata=$METADATA")

if echo $UPLOAD_RESPONSE | grep -q "id"; then
  FILE_ID=$(echo $UPLOAD_RESPONSE | grep -o '"id":"[^"]*' | cut -d'"' -f4)
  echo "✅ File upload successful"
  echo "   File ID: $FILE_ID"
else
  echo "❌ File upload failed"
  echo $UPLOAD_RESPONSE
  rm /tmp/$TEST_FILE
  exit 1
fi

# Test 6: List files again (should have one file)
echo ""
echo "6. Testing file list (should have 1 file)..."
FILES_RESPONSE=$(curl -s -X GET "$API_BASE/api/files" \
  -H "Authorization: Bearer $TOKEN")

if echo $FILES_RESPONSE | grep -q "$TEST_FILE"; then
  echo "✅ File appears in list"
else
  echo "❌ File not found in list"
  echo $FILES_RESPONSE
fi

# Test 7: Search for file
echo ""
echo "7. Testing file search..."
SEARCH_RESPONSE=$(curl -s -X GET "$API_BASE/api/files?q=test_file" \
  -H "Authorization: Bearer $TOKEN")

if echo $SEARCH_RESPONSE | grep -q "$TEST_FILE"; then
  echo "✅ File search works"
else
  echo "❌ File search failed"
fi

# Test 8: Download file
echo ""
echo "8. Testing file download..."
DOWNLOAD_RESPONSE=$(curl -s -X GET "$API_BASE/api/files/$FILE_ID/download" \
  -H "Authorization: Bearer $TOKEN" \
  -w "%{http_code}" \
  -o /tmp/downloaded_$TEST_FILE)

HTTP_CODE="${DOWNLOAD_RESPONSE: -3}"
if [ "$HTTP_CODE" = "200" ]; then
  echo "✅ File download successful"
else
  echo "❌ File download failed (HTTP $HTTP_CODE)"
fi

# Test 9: Delete file
echo ""
echo "9. Testing file deletion..."
DELETE_RESPONSE=$(curl -s -X DELETE "$API_BASE/api/files/$FILE_ID" \
  -H "Authorization: Bearer $TOKEN" \
  -w "%{http_code}")

HTTP_CODE="${DELETE_RESPONSE: -3}"
if [ "$HTTP_CODE" = "204" ]; then
  echo "✅ File deletion successful"
else
  echo "❌ File deletion failed (HTTP $HTTP_CODE)"
  echo $DELETE_RESPONSE
fi

# Test 10: Verify file is gone
echo ""
echo "10. Verifying file was deleted..."
FILES_RESPONSE=$(curl -s -X GET "$API_BASE/api/files" \
  -H "Authorization: Bearer $TOKEN")

if echo $FILES_RESPONSE | grep -q "^\[\]$"; then
  echo "✅ File list is empty again"
else
  echo "⚠️  File list: $FILES_RESPONSE"
fi

# Cleanup
rm -f /tmp/$TEST_FILE /tmp/downloaded_$TEST_FILE

echo ""
echo "=== All tests completed! ==="
echo ""
echo "Next steps:"
echo "1. Open http://localhost:3000 in your browser"
echo "2. Sign up with a username and password"
echo "3. Upload a file (it will be encrypted in the browser)"
echo "4. Download the file (you'll need the password to decrypt)"
echo ""
echo "Check the Swagger UI at: http://localhost:3000/swagger-ui"
