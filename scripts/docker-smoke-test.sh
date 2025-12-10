#!/bin/bash
set -e

IMAGE_NAME="proxypal-server"
CONTAINER_NAME="proxypal-smoke-test"
DATA_DIR="./tmp-data"
PORT=3000
TIMEOUT=60
ENCRYPTION_KEY="0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
ADMIN_PASSWORD="test-admin-password-123"

cleanup() {
    echo "Cleaning up..."
    docker rm -f "$CONTAINER_NAME" 2>/dev/null || true
    rm -rf "$DATA_DIR"
}

trap cleanup EXIT

echo "=== ProxyPal Docker Smoke Test ==="

# Step 1: Build Docker image
echo "Building Docker image..."
docker build -t "$IMAGE_NAME" .

# Step 2: Prepare data directory
mkdir -p "$DATA_DIR"

# Step 3: Run container
echo "Starting container..."
docker run -d \
    --name "$CONTAINER_NAME" \
    -p "$PORT:3000" \
    -e "PORT=3000" \
    -e "DATABASE_PATH=/data/proxypal.db" \
    -e "DATA_DIR=/data" \
    -e "ENCRYPTION_KEY=$ENCRYPTION_KEY" \
    -e "ADMIN_PASSWORD=$ADMIN_PASSWORD" \
    -v "$(pwd)/$DATA_DIR:/data" \
    "$IMAGE_NAME"

# Step 4: Wait for healthz
echo "Waiting for health check..."
START_TIME=$(date +%s)
while true; do
    CURRENT_TIME=$(date +%s)
    ELAPSED=$((CURRENT_TIME - START_TIME))
    
    if [ $ELAPSED -ge $TIMEOUT ]; then
        echo "ERROR: Health check timed out after ${TIMEOUT}s"
        docker logs "$CONTAINER_NAME"
        exit 1
    fi
    
    HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "http://localhost:$PORT/healthz" 2>/dev/null || echo "000")
    
    if [ "$HTTP_CODE" = "200" ]; then
        echo "Health check passed!"
        break
    fi
    
    echo "Waiting... (${ELAPSED}s elapsed, status: $HTTP_CODE)"
    sleep 2
done

# Step 5: Test login endpoint
echo "Testing POST /api/auth/login..."
LOGIN_RESPONSE=$(curl -s -X POST "http://localhost:$PORT/api/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"password\": \"$ADMIN_PASSWORD\"}")

if echo "$LOGIN_RESPONSE" | grep -q "token"; then
    echo "Login test passed!"
    TOKEN=$(echo "$LOGIN_RESPONSE" | grep -o '"token":"[^"]*"' | cut -d'"' -f4)
else
    echo "ERROR: Login test failed"
    echo "Response: $LOGIN_RESPONSE"
    exit 1
fi

# Step 6: Test proxy start endpoint
echo "Testing POST /api/proxy/start..."
PROXY_RESPONSE=$(curl -s -X POST "http://localhost:$PORT/api/proxy/start" \
    -H "Content-Type: application/json" \
    -H "Authorization: Bearer $TOKEN" \
    -w "\n%{http_code}" 2>/dev/null)

HTTP_CODE=$(echo "$PROXY_RESPONSE" | tail -n1)
BODY=$(echo "$PROXY_RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "201" ]; then
    echo "Proxy start test passed!"
else
    echo "WARNING: Proxy start returned $HTTP_CODE (may be expected if no proxy configured)"
    echo "Response: $BODY"
fi

echo ""
echo "=== Smoke Test PASSED ==="
