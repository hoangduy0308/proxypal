#!/bin/bash
#
# ProxyPal Server - Post-Deployment Smoke Test
#
# Usage: ./post-deploy-smoke.sh <BASE_URL> <ADMIN_PASSWORD>
# Example: ./post-deploy-smoke.sh https://proxypal-server.onrender.com mypassword123
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Parameters
BASE_URL="${1:-}"
ADMIN_PASSWORD="${2:-}"

if [[ -z "$BASE_URL" ]] || [[ -z "$ADMIN_PASSWORD" ]]; then
    echo "Usage: $0 <BASE_URL> <ADMIN_PASSWORD>"
    echo "Example: $0 https://proxypal-server.onrender.com mypassword123"
    exit 1
fi

# Remove trailing slash from BASE_URL
BASE_URL="${BASE_URL%/}"

# Test results
PASSED=0
FAILED=0
COOKIE_JAR=$(mktemp)

cleanup() {
    rm -f "$COOKIE_JAR"
}
trap cleanup EXIT

log_pass() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
    ((PASSED++))
}

log_fail() {
    echo -e "${RED}✗ FAIL${NC}: $1"
    echo -e "  ${YELLOW}Details${NC}: $2"
    ((FAILED++))
}

echo "=================================="
echo "ProxyPal Post-Deploy Smoke Test"
echo "=================================="
echo "Base URL: $BASE_URL"
echo ""

# Test 1: Health Check
echo "Test 1: Health Check (/healthz)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/healthz" 2>&1) || true
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "200" ]]; then
    if echo "$BODY" | grep -q '"status"'; then
        log_pass "Health check returned 200 with status"
    else
        log_fail "Health check returned 200 but missing status in body" "$BODY"
    fi
else
    log_fail "Health check failed with HTTP $HTTP_CODE" "$BODY"
fi

# Test 2: Admin Login
echo ""
echo "Test 2: Admin Login (/api/auth/login)"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$BASE_URL/api/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"password\":\"$ADMIN_PASSWORD\"}" \
    -c "$COOKIE_JAR" 2>&1) || true
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "200" ]]; then
    log_pass "Admin login successful"
else
    log_fail "Admin login failed with HTTP $HTTP_CODE" "$BODY"
fi

# Test 3: Auth Status
echo ""
echo "Test 3: Auth Status (/api/auth/status)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/auth/status" \
    -b "$COOKIE_JAR" 2>&1) || true
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "200" ]]; then
    if echo "$BODY" | grep -q 'authenticated\|true'; then
        log_pass "Auth status shows authenticated"
    else
        log_fail "Auth status returned 200 but not authenticated" "$BODY"
    fi
else
    log_fail "Auth status failed with HTTP $HTTP_CODE" "$BODY"
fi

# Test 4: Users API
echo ""
echo "Test 4: Users API (/api/users)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/users" \
    -b "$COOKIE_JAR" 2>&1) || true
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "200" ]]; then
    log_pass "Users API returned 200"
else
    log_fail "Users API failed with HTTP $HTTP_CODE" "$BODY"
fi

# Test 5: Providers API
echo ""
echo "Test 5: Providers API (/api/providers)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/providers" \
    -b "$COOKIE_JAR" 2>&1) || true
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "200" ]]; then
    log_pass "Providers API returned 200"
else
    log_fail "Providers API failed with HTTP $HTTP_CODE" "$BODY"
fi

# Test 6: Proxy Status
echo ""
echo "Test 6: Proxy Status (/api/proxy/status)"
RESPONSE=$(curl -s -w "\n%{http_code}" "$BASE_URL/api/proxy/status" \
    -b "$COOKIE_JAR" 2>&1) || true
HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | sed '$d')

if [[ "$HTTP_CODE" == "200" ]]; then
    log_pass "Proxy status API returned 200"
else
    log_fail "Proxy status API failed with HTTP $HTTP_CODE" "$BODY"
fi

# Summary
echo ""
echo "=================================="
echo "Test Summary"
echo "=================================="
echo -e "${GREEN}Passed${NC}: $PASSED"
echo -e "${RED}Failed${NC}: $FAILED"
echo ""

if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}Some tests failed!${NC}"
    exit 1
else
    echo -e "${GREEN}All tests passed!${NC}"
    exit 0
fi
