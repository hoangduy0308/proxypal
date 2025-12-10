# ProxyPal Server - API Design

## Base URL

- **Development:** `http://localhost:3000`
- **Production:** `https://your-app.onrender.com`

## Authentication

### Admin Authentication

All `/api/*` endpoints (except `/api/auth/login`) require admin authentication.

**Headers:**
```
Cookie: session=<session_token>
```

### User API Authentication

Proxy endpoints require user API key.

**Headers:**
```
Authorization: Bearer sk-username-xxxxxxxx
```

---

## API Endpoints

### Health Check

#### `GET /healthz`

Check server and proxy health.

**Response:**
```json
{
  "ok": true,
  "proxy_running": true,
  "uptime_seconds": 3600,
  "version": "1.0.0"
}
```

---

### Authentication

#### `POST /api/auth/login`

Admin login.

**Request:**
```json
{
  "password": "your-admin-password"
}
```

**Response (200):**
```json
{
  "success": true,
  "message": "Logged in successfully"
}
```

**Response (401):**
```json
{
  "success": false,
  "error": "Invalid password"
}
```

**Cookies Set:**
```
Set-Cookie: session=<token>; HttpOnly; Secure; SameSite=Strict; Path=/
```

---

#### `POST /api/auth/logout`

Admin logout.

**Response:**
```json
{
  "success": true
}
```

---

#### `GET /api/auth/status`

Check authentication status.

**Response (authenticated):**
```json
{
  "authenticated": true,
  "expires_at": "2024-01-15T12:00:00Z"
}
```

**Response (not authenticated):**
```json
{
  "authenticated": false
}
```

---

### Proxy Management

#### `GET /api/proxy/status`

Get proxy status.

**Response:**
```json
{
  "running": true,
  "pid": 12345,
  "port": 8317,
  "uptime_seconds": 7200,
  "total_requests": 1523,
  "active_providers": ["claude", "openai"]
}
```

---

#### `POST /api/proxy/start`

Start the proxy.

**Response (200):**
```json
{
  "success": true,
  "pid": 12345,
  "port": 8317
}
```

**Response (409 - already running):**
```json
{
  "success": false,
  "error": "Proxy is already running"
}
```

---

#### `POST /api/proxy/stop`

Stop the proxy.

**Response:**
```json
{
  "success": true
}
```

---

#### `POST /api/proxy/restart`

Restart the proxy (stop then start).

**Response:**
```json
{
  "success": true,
  "pid": 12346,
  "port": 8317
}
```

---

### User Management

#### `GET /api/users`

List all users.

**Query Parameters:**
- `page` (optional): Page number, default 1
- `limit` (optional): Items per page, default 50

**Response:**
```json
{
  "users": [
    {
      "id": 1,
      "name": "john",
      "api_key_prefix": "sk-john",
      "quota_tokens": 1000000,
      "used_tokens": 250000,
      "enabled": true,
      "created_at": "2024-01-01T00:00:00Z",
      "last_used_at": "2024-01-14T15:30:00Z"
    },
    {
      "id": 2,
      "name": "jane",
      "api_key_prefix": "sk-jane",
      "quota_tokens": null,
      "used_tokens": 500000,
      "enabled": true,
      "created_at": "2024-01-05T00:00:00Z",
      "last_used_at": "2024-01-14T16:00:00Z"
    }
  ],
  "total": 2,
  "page": 1,
  "limit": 50
}
```

---

#### `POST /api/users`

Create a new user.

**Request:**
```json
{
  "name": "alice",
  "quota_tokens": 500000
}
```

**Response (201):**
```json
{
  "id": 3,
  "name": "alice",
  "api_key": "sk-alice-a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6",
  "api_key_prefix": "sk-alice",
  "quota_tokens": 500000,
  "used_tokens": 0,
  "enabled": true,
  "created_at": "2024-01-15T10:00:00Z"
}
```

> **Note:** `api_key` is only returned once at creation. Store it securely!

**Response (400):**
```json
{
  "success": false,
  "error": "User name already exists"
}
```

---

#### `GET /api/users/:id`

Get user details.

**Response:**
```json
{
  "id": 1,
  "name": "john",
  "api_key_prefix": "sk-john",
  "quota_tokens": 1000000,
  "used_tokens": 250000,
  "enabled": true,
  "created_at": "2024-01-01T00:00:00Z",
  "last_used_at": "2024-01-14T15:30:00Z",
  "usage_this_month": {
    "requests": 523,
    "tokens_input": 150000,
    "tokens_output": 100000
  }
}
```

---

#### `PUT /api/users/:id`

Update user.

**Request:**
```json
{
  "name": "john_updated",
  "quota_tokens": 2000000,
  "enabled": true
}
```

**Response:**
```json
{
  "id": 1,
  "name": "john_updated",
  "api_key_prefix": "sk-john",
  "quota_tokens": 2000000,
  "used_tokens": 250000,
  "enabled": true,
  "created_at": "2024-01-01T00:00:00Z",
  "last_used_at": "2024-01-14T15:30:00Z"
}
```

---

#### `DELETE /api/users/:id`

Delete user.

**Response:**
```json
{
  "success": true
}
```

---

#### `POST /api/users/:id/regenerate-key`

Generate a new API key for user.

**Response:**
```json
{
  "api_key": "sk-john-x9y8z7w6v5u4t3s2r1q0p9o8n7m6l5k4",
  "api_key_prefix": "sk-john"
}
```

> **Warning:** Old API key becomes invalid immediately.

---

#### `POST /api/users/:id/reset-usage`

Reset user's usage counter.

**Response:**
```json
{
  "success": true,
  "previous_used_tokens": 250000
}
```

---

### Provider Management

#### `GET /api/providers`

List all configured providers.

**Response:**
```json
{
  "providers": [
    {
      "name": "claude",
      "type": "oauth",
      "status": "active",
      "accounts": 2,
      "models": ["claude-3-opus", "claude-3-sonnet", "claude-3-haiku"],
      "last_used_at": "2024-01-14T16:00:00Z"
    },
    {
      "name": "openai",
      "type": "api_key",
      "status": "active",
      "accounts": 1,
      "models": ["gpt-4", "gpt-4-turbo", "gpt-3.5-turbo"],
      "last_used_at": "2024-01-14T15:00:00Z"
    },
    {
      "name": "gemini",
      "type": "oauth",
      "status": "inactive",
      "accounts": 0,
      "models": [],
      "last_used_at": null
    }
  ]
}
```

---

#### `GET /api/providers/:name`

Get provider details.

**Response:**
```json
{
  "name": "claude",
  "type": "oauth",
  "status": "active",
  "accounts": [
    {
      "id": 1,
      "email": "user1@example.com",
      "status": "active",
      "added_at": "2024-01-01T00:00:00Z",
      "expires_at": "2024-02-01T00:00:00Z"
    },
    {
      "id": 2,
      "email": "user2@example.com",
      "status": "active",
      "added_at": "2024-01-10T00:00:00Z",
      "expires_at": "2024-02-10T00:00:00Z"
    }
  ],
  "settings": {
    "load_balancing": "round_robin",
    "timeout_seconds": 120
  }
}
```

---

#### `DELETE /api/providers/:name/accounts/:account_id`

Remove a provider account.

**Response:**
```json
{
  "success": true
}
```

---

#### `PUT /api/providers/:name/settings`

Update provider settings.

**Request:**
```json
{
  "load_balancing": "least_used",
  "timeout_seconds": 180
}
```

**Response:**
```json
{
  "success": true
}
```

---

#### `GET /api/providers/:name/health`

Check provider health.

**Response:**
```json
{
  "name": "claude",
  "healthy": true,
  "accounts": [
    { "id": 1, "email": "user1@example.com", "healthy": true },
    { "id": 2, "email": "user2@example.com", "healthy": false, "error": "Token expired" }
  ],
  "checked_at": "2024-01-15T10:00:00Z"
}
```

---

### OAuth

#### `GET /oauth/:provider/start`

Start OAuth flow for a provider.

**Supported providers:** `claude`, `chatgpt`, `gemini`, `copilot`

**Response:** Redirect to provider's OAuth page.

**Example:**
```
GET /oauth/claude/start
→ 302 Redirect to https://claude.ai/oauth/authorize?client_id=...&redirect_uri=...&state=...
```

---

#### `GET /oauth/:provider/callback`

OAuth callback handler.

**Query Parameters:**
- `code`: Authorization code from provider
- `state`: State token for CSRF protection

**Response:** Redirect to dashboard with success/error message.

**Success:**
```
302 Redirect to /?oauth=success&provider=claude
```

**Error:**
```
302 Redirect to /?oauth=error&provider=claude&message=Token+exchange+failed
```

---

### Usage Statistics

#### `GET /api/usage`

Get overall usage statistics.

**Query Parameters:**
- `period`: `today`, `week`, `month`, `all` (default: `month`)

**Response:**
```json
{
  "period": "month",
  "total_requests": 15230,
  "total_tokens_input": 5000000,
  "total_tokens_output": 3000000,
  "by_provider": {
    "claude": {
      "requests": 10000,
      "tokens_input": 3500000,
      "tokens_output": 2000000
    },
    "openai": {
      "requests": 5230,
      "tokens_input": 1500000,
      "tokens_output": 1000000
    }
  },
  "by_user": {
    "john": {
      "requests": 8000,
      "tokens_input": 2500000,
      "tokens_output": 1500000
    },
    "jane": {
      "requests": 7230,
      "tokens_input": 2500000,
      "tokens_output": 1500000
    }
  }
}
```

---

#### `GET /api/usage/users/:id`

Get usage for a specific user.

**Query Parameters:**
- `period`: `today`, `week`, `month`, `all`

**Response:**
```json
{
  "user_id": 1,
  "user_name": "john",
  "period": "month",
  "total_requests": 8000,
  "total_tokens_input": 2500000,
  "total_tokens_output": 1500000,
  "by_provider": {
    "claude": {
      "requests": 5000,
      "tokens_input": 1500000,
      "tokens_output": 1000000
    },
    "openai": {
      "requests": 3000,
      "tokens_input": 1000000,
      "tokens_output": 500000
    }
  },
  "by_model": {
    "claude-3-opus": { "requests": 2000, "tokens": 800000 },
    "claude-3-sonnet": { "requests": 3000, "tokens": 700000 },
    "gpt-4": { "requests": 3000, "tokens": 1000000 }
  },
  "daily": [
    { "date": "2024-01-14", "requests": 500, "tokens": 150000 },
    { "date": "2024-01-13", "requests": 450, "tokens": 140000 }
  ]
}
```

---

#### `GET /api/usage/daily`

Get daily usage breakdown.

**Query Parameters:**
- `days`: Number of days (default: 30, max: 90)
- `user_id`: Filter by user (optional)
- `provider`: Filter by provider (optional)

**Response:**
```json
{
  "days": 7,
  "data": [
    {
      "date": "2024-01-15",
      "requests": 2100,
      "tokens_input": 700000,
      "tokens_output": 400000
    },
    {
      "date": "2024-01-14",
      "requests": 2300,
      "tokens_input": 750000,
      "tokens_output": 450000
    }
  ]
}
```

---

### Configuration

#### `GET /api/config`

Get current configuration.

**Response:**
```json
{
  "proxy_port": 8317,
  "admin_port": 3000,
  "log_level": "info",
  "auto_start_proxy": true,
  "model_mappings": {
    "gpt-4": "claude-3-opus",
    "gpt-3.5-turbo": "claude-3-haiku"
  },
  "rate_limits": {
    "requests_per_minute": 60,
    "tokens_per_day": 1000000
  }
}
```

---

#### `PUT /api/config`

Update configuration.

**Request:**
```json
{
  "proxy_port": 8318,
  "log_level": "debug",
  "auto_start_proxy": false,
  "model_mappings": {
    "gpt-4": "claude-3-opus"
  }
}
```

**Response:**
```json
{
  "success": true,
  "restart_required": true
}
```

---

### Logs

#### `GET /api/logs`

Get recent request logs.

**Query Parameters:**
- `limit`: Number of logs (default: 100, max: 1000)
- `offset`: Pagination offset
- `user_id`: Filter by user
- `provider`: Filter by provider
- `status`: Filter by status (`success`, `error`)

**Response:**
```json
{
  "logs": [
    {
      "id": 1523,
      "timestamp": "2024-01-15T10:30:00Z",
      "user_id": 1,
      "user_name": "john",
      "provider": "claude",
      "model": "claude-3-opus",
      "tokens_input": 500,
      "tokens_output": 1200,
      "duration_ms": 2500,
      "status": "success"
    }
  ],
  "total": 15230,
  "limit": 100,
  "offset": 0
}
```

---

#### `WebSocket /ws/logs` (Phase 6 - Future)

> ⚠️ **Note**: This endpoint is planned for Phase 6 (Future) and may not be available in the initial deployment. See IMPLEMENTATION_PLAN.md for details.

Real-time log streaming.

**Message Format (server → client):**
```json
{
  "type": "log",
  "data": {
    "id": 1524,
    "timestamp": "2024-01-15T10:30:05Z",
    "user_name": "john",
    "provider": "claude",
    "model": "claude-3-opus",
    "tokens_input": 300,
    "tokens_output": 800,
    "duration_ms": 1800,
    "status": "success"
  }
}
```

---

## Proxy API (OpenAI-Compatible)

These endpoints are accessed by end users with their API keys.

### Base URL

`https://your-app.onrender.com` or configured proxy port.

### Endpoints

Follows OpenAI API specification:

#### `POST /v1/chat/completions`

**Headers:**
```
Authorization: Bearer sk-username-xxxxxxxx
Content-Type: application/json
```

**Request:**
```json
{
  "model": "gpt-4",
  "messages": [
    { "role": "user", "content": "Hello!" }
  ]
}
```

**Response:** Standard OpenAI response format.

---

#### `GET /v1/models`

List available models.

**Response:**
```json
{
  "object": "list",
  "data": [
    { "id": "gpt-4", "object": "model" },
    { "id": "gpt-3.5-turbo", "object": "model" },
    { "id": "claude-3-opus", "object": "model" }
  ]
}
```

---

## Error Responses

All errors follow this format:

```json
{
  "success": false,
  "error": "Error message here",
  "code": "ERROR_CODE"
}
```

### Error Codes

| Code | HTTP Status | Description |
|------|-------------|-------------|
| `UNAUTHORIZED` | 401 | Not authenticated |
| `FORBIDDEN` | 403 | Not authorized for this action |
| `NOT_FOUND` | 404 | Resource not found |
| `VALIDATION_ERROR` | 400 | Invalid request data |
| `CONFLICT` | 409 | Resource conflict |
| `QUOTA_EXCEEDED` | 429 | User quota exceeded |
| `RATE_LIMITED` | 429 | Too many requests |
| `PROVIDER_ERROR` | 502 | AI provider error |
| `INTERNAL_ERROR` | 500 | Server error |

---

## Rate Limiting

### Admin API
- 100 requests per minute per session

### Proxy API
- Configurable per user
- Default: 60 requests per minute
- Quota: Configurable token limit

### Headers
```
X-RateLimit-Limit: 60
X-RateLimit-Remaining: 45
X-RateLimit-Reset: 1705312800
```
