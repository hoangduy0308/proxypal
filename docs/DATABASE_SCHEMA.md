# ProxyPal Server - Database Schema

## Overview

ProxyPal Server sử dụng **SQLite** làm database chính vì:
- Đơn giản, không cần server riêng
- Phù hợp với quy mô nhỏ (~5-10 users)
- Dễ backup (1 file)
- Tích hợp tốt với Render Disk

> ⚠️ **IMPORTANT**: SQLite + Render Disk chỉ hỗ trợ **single instance deployment**. Không thể horizontal scale. Nếu cần scale, migrate sang PostgreSQL.

## Database File Location

```
/data/proxypal.db          # Production (Render Disk)
~/.proxypal/proxypal.db    # Development
```

---

## DateTime Storage Convention

> **Convention**: Store timestamps as **RFC3339 strings** for compatibility with Rust `DateTime<Utc>`.

```sql
-- Use strftime for RFC3339 format (ISO 8601)
DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now'))

-- Example: "2024-01-15T10:30:00Z"
```

**Rust parsing:**
```rust
use chrono::{DateTime, Utc};

// Parse from SQLite TEXT column
let timestamp: DateTime<Utc> = row.get::<_, String>("created_at")?
    .parse()
    .expect("Invalid timestamp format");
```

---

## Tables

### 1. `users` - User Management

Quản lý users và API keys.

```sql
CREATE TABLE users (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,
    api_key_hash    TEXT NOT NULL,           -- Argon2 hash of API key
    api_key_prefix  TEXT NOT NULL,           -- "sk-john" for display
    quota_tokens    INTEGER DEFAULT NULL,    -- NULL = unlimited
    used_tokens     INTEGER DEFAULT 0,
    enabled         INTEGER DEFAULT 1,       -- 0 = disabled, 1 = enabled
    created_at      TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%SZ', 'now')),
    last_used_at    TEXT DEFAULT NULL,
    
    -- Indexes
    UNIQUE(api_key_prefix)
);

CREATE INDEX idx_users_api_key_prefix ON users(api_key_prefix);
CREATE INDEX idx_users_enabled ON users(enabled);
```

**Example Data:**
```
| id | name  | api_key_prefix | quota_tokens | used_tokens | enabled |
|----|-------|----------------|--------------|-------------|---------|
| 1  | john  | sk-john        | 1000000      | 250000      | 1       |
| 2  | jane  | sk-jane        | NULL         | 500000      | 1       |
| 3  | guest | sk-guest       | 100000       | 100000      | 0       |
```

---

### 2. `usage_logs` - Request Logging

Log mỗi request để tracking usage.

```sql
CREATE TABLE usage_logs (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id         INTEGER NOT NULL,
    provider        TEXT NOT NULL,           -- "claude", "openai", "gemini"
    model           TEXT NOT NULL,           -- "claude-3-opus", "gpt-4"
    tokens_input    INTEGER NOT NULL DEFAULT 0,
    tokens_output   INTEGER NOT NULL DEFAULT 0,
    duration_ms     INTEGER NOT NULL DEFAULT 0,
    status          TEXT NOT NULL DEFAULT 'success',  -- "success", "error"
    error_message   TEXT DEFAULT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

CREATE INDEX idx_usage_logs_user_id ON usage_logs(user_id);
CREATE INDEX idx_usage_logs_created_at ON usage_logs(created_at);
CREATE INDEX idx_usage_logs_provider ON usage_logs(provider);
CREATE INDEX idx_usage_logs_user_date ON usage_logs(user_id, created_at);
```

**Example Data:**
```
| id   | user_id | provider | model         | tokens_input | tokens_output | status  |
|------|---------|----------|---------------|--------------|---------------|---------|
| 1523 | 1       | claude   | claude-3-opus | 500          | 1200          | success |
| 1524 | 2       | openai   | gpt-4         | 300          | 800           | success |
| 1525 | 1       | claude   | claude-3-opus | 400          | 0             | error   |
```

---

### 3. `providers` - AI Provider Configuration

Lưu thông tin các AI providers.

```sql
CREATE TABLE providers (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT NOT NULL UNIQUE,    -- "claude", "openai", "gemini"
    type            TEXT NOT NULL,           -- "oauth", "api_key"
    enabled         INTEGER DEFAULT 1,
    settings        TEXT DEFAULT '{}',       -- JSON settings
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Example Data:**
```
| id | name    | type    | enabled | settings                              |
|----|---------|---------|---------|---------------------------------------|
| 1  | claude  | oauth   | 1       | {"load_balancing": "round_robin"}    |
| 2  | openai  | api_key | 1       | {"timeout_seconds": 120}             |
| 3  | gemini  | oauth   | 0       | {}                                    |
```

---

### 4. `provider_accounts` - Provider OAuth Accounts

Lưu các accounts đã OAuth cho mỗi provider.

```sql
CREATE TABLE provider_accounts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    provider_id     INTEGER NOT NULL,
    email           TEXT,                    -- User email from OAuth
    tokens          TEXT NOT NULL,           -- Encrypted JSON tokens
    status          TEXT DEFAULT 'active',   -- "active", "expired", "revoked"
    expires_at      TEXT DEFAULT NULL,
    last_used_at    TEXT DEFAULT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    
    FOREIGN KEY (provider_id) REFERENCES providers(id) ON DELETE CASCADE
);

CREATE INDEX idx_provider_accounts_provider_id ON provider_accounts(provider_id);
CREATE INDEX idx_provider_accounts_status ON provider_accounts(status);
```

**Token Encryption:**
```rust
// tokens column stores encrypted JSON:
{
    "access_token": "encrypted...",
    "refresh_token": "encrypted...",
    "session_key": "encrypted..."
}
```

**Example Data:**
```
| id | provider_id | email             | status  | expires_at          |
|----|-------------|-------------------|---------|---------------------|
| 1  | 1           | user1@example.com | active  | 2024-02-01 00:00:00 |
| 2  | 1           | user2@example.com | active  | 2024-02-10 00:00:00 |
| 3  | 2           | NULL              | active  | NULL                |
```

---

### 5. `sessions` - Admin Sessions

Quản lý admin login sessions.

```sql
CREATE TABLE sessions (
    id              TEXT PRIMARY KEY,        -- UUID session token
    expires_at      TEXT NOT NULL,
    created_at      TEXT NOT NULL DEFAULT (datetime('now')),
    last_accessed   TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE INDEX idx_sessions_expires_at ON sessions(expires_at);
```

**Session Cleanup:**
```sql
-- Run periodically to clean expired sessions
DELETE FROM sessions WHERE datetime(expires_at) < datetime('now');
```

---

### 6. `settings` - Application Settings

Key-value store cho app settings.

```sql
CREATE TABLE settings (
    key             TEXT PRIMARY KEY,
    value           TEXT NOT NULL,
    updated_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
```

**Example Data:**
```
| key                  | value                                    |
|----------------------|------------------------------------------|
| proxy_port           | 8317                                     |
| admin_password_hash  | $argon2id$v=19$m=19456...               |
| auto_start_proxy     | true                                     |
| model_mappings       | {"gpt-4": "claude-3-opus"}              |
| rate_limit_rpm       | 60                                       |
```

---

### 7. `daily_usage` - Aggregated Daily Stats

Pre-aggregated daily usage để query nhanh hơn.

```sql
CREATE TABLE daily_usage (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    date            TEXT NOT NULL,           -- "2024-01-15"
    user_id         INTEGER,                 -- NULL = all users
    provider        TEXT,                    -- NULL = all providers
    requests        INTEGER DEFAULT 0,
    tokens_input    INTEGER DEFAULT 0,
    tokens_output   INTEGER DEFAULT 0,
    
    UNIQUE(date, user_id, provider)
);

CREATE INDEX idx_daily_usage_date ON daily_usage(date);
CREATE INDEX idx_daily_usage_user_id ON daily_usage(user_id);
```

> ⚠️ **Note on User Deletion**: `usage_logs` has `ON DELETE CASCADE` from `users`. Deleting a user will **permanently delete** all their usage history. If you need to keep historical stats, consider soft-delete (set `enabled = 0`) instead.

**Aggregation Job:**
```sql
-- Run daily to aggregate yesterday's usage
INSERT OR REPLACE INTO daily_usage (date, user_id, provider, requests, tokens_input, tokens_output)
SELECT 
    date(created_at) as date,
    user_id,
    provider,
    COUNT(*) as requests,
    SUM(tokens_input) as tokens_input,
    SUM(tokens_output) as tokens_output
FROM usage_logs
WHERE date(created_at) = date('now', '-1 day')
GROUP BY date(created_at), user_id, provider;
```

---

## Migrations

### Migration System

```sql
CREATE TABLE migrations (
    id              INTEGER PRIMARY KEY,
    name            TEXT NOT NULL,
    applied_at      TEXT NOT NULL DEFAULT (datetime('now'))
);
```

### Migration Files

```
proxypal-server/src/db/migrations/
├── 001_initial.sql
├── 002_add_daily_usage.sql
└── 003_add_rate_limits.sql
```

**Example Migration:**
```sql
-- 001_initial.sql
CREATE TABLE IF NOT EXISTS users (...);
CREATE TABLE IF NOT EXISTS usage_logs (...);
CREATE TABLE IF NOT EXISTS providers (...);
CREATE TABLE IF NOT EXISTS provider_accounts (...);
CREATE TABLE IF NOT EXISTS sessions (...);
CREATE TABLE IF NOT EXISTS settings (...);

INSERT INTO migrations (id, name) VALUES (1, '001_initial');
```

---

## Entity Relationship Diagram

```
┌─────────────────┐
│     users       │
├─────────────────┤
│ id (PK)         │
│ name            │
│ api_key_hash    │
│ api_key_prefix  │
│ quota_tokens    │
│ used_tokens     │
│ enabled         │
└────────┬────────┘
         │
         │ 1:N
         ▼
┌─────────────────┐
│   usage_logs    │
├─────────────────┤
│ id (PK)         │
│ user_id (FK)    │───────────────┐
│ provider        │               │
│ model           │               │
│ tokens_input    │               │
│ tokens_output   │               │
│ status          │               │
└─────────────────┘               │
                                  │
┌─────────────────┐               │
│   providers     │               │
├─────────────────┤               │
│ id (PK)         │               │
│ name            │◄──────────────┤ (logical ref)
│ type            │               │
│ enabled         │               │
│ settings        │               │
└────────┬────────┘               │
         │                        │
         │ 1:N                    │
         ▼                        │
┌─────────────────┐               │
│provider_accounts│               │
├─────────────────┤               │
│ id (PK)         │               │
│ provider_id(FK) │               │
│ email           │               │
│ tokens (enc)    │               │
│ status          │               │
└─────────────────┘               │
                                  │
┌─────────────────┐               │
│  daily_usage    │               │
├─────────────────┤               │
│ id (PK)         │               │
│ date            │               │
│ user_id         │───────────────┘
│ provider        │
│ requests        │
│ tokens_input    │
│ tokens_output   │
└─────────────────┘

┌─────────────────┐     ┌─────────────────┐
│    sessions     │     │    settings     │
├─────────────────┤     ├─────────────────┤
│ id (PK)         │     │ key (PK)        │
│ expires_at      │     │ value           │
│ created_at      │     │ updated_at      │
└─────────────────┘     └─────────────────┘
```

---

## Rust Models

```rust
// src/db/models.rs

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub api_key_hash: String,
    pub api_key_prefix: String,
    pub quota_tokens: Option<i64>,
    pub used_tokens: i64,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UsageLog {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub model: String,
    pub tokens_input: i64,
    pub tokens_output: i64,
    pub duration_ms: i64,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub settings: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderAccount {
    pub id: i64,
    pub provider_id: i64,
    pub email: Option<String>,
    pub tokens: String,  // Encrypted
    pub status: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    pub id: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_accessed: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DailyUsage {
    pub id: i64,
    pub date: String,
    pub user_id: Option<i64>,
    pub provider: Option<String>,
    pub requests: i64,
    pub tokens_input: i64,
    pub tokens_output: i64,
}
```

---

## Common Queries

### User Authentication

> **IMPORTANT**: API key authentication must verify the **full API key hash**, not just the prefix!

```sql
-- Step 1: Extract prefix from API key (e.g., "sk-john" from "sk-john-a1b2c3...")
-- This is done in application code, not SQL

-- Step 2: Look up user by prefix (for performance)
SELECT id, name, api_key_hash, quota_tokens, used_tokens, enabled
FROM users
WHERE api_key_prefix = ?
  AND enabled = 1;

-- Step 3: In Rust, verify the full API key against api_key_hash using Argon2
-- if !argon2::verify_encoded(&user.api_key_hash, full_api_key.as_bytes())? {
--     return Err(AuthError::InvalidApiKey);
-- }

-- Step 4: Update last used (only after successful auth)
UPDATE users SET last_used_at = datetime('now') WHERE id = ?;
```

**Rust Implementation:**
```rust
use argon2::{Argon2, PasswordHash, PasswordVerifier};

fn verify_api_key(api_key: &str, stored_hash: &str) -> Result<bool, Error> {
    let parsed_hash = PasswordHash::new(stored_hash)?;
    Ok(Argon2::default()
        .verify_password(api_key.as_bytes(), &parsed_hash)
        .is_ok())
}
```

### Usage Tracking

```sql
-- Log request
INSERT INTO usage_logs (user_id, provider, model, tokens_input, tokens_output, duration_ms, status)
VALUES (?, ?, ?, ?, ?, ?, ?);

-- Update user's used tokens
UPDATE users 
SET used_tokens = used_tokens + ? 
WHERE id = ?;
```

### Usage Statistics

```sql
-- Get user usage for current month
SELECT 
    COUNT(*) as requests,
    SUM(tokens_input) as tokens_input,
    SUM(tokens_output) as tokens_output
FROM usage_logs
WHERE user_id = ?
  AND created_at >= date('now', 'start of month');

-- Get usage by provider
SELECT 
    provider,
    COUNT(*) as requests,
    SUM(tokens_input) as tokens_input,
    SUM(tokens_output) as tokens_output
FROM usage_logs
WHERE created_at >= date('now', '-30 days')
GROUP BY provider;

-- Get daily usage for chart
SELECT 
    date(created_at) as date,
    COUNT(*) as requests,
    SUM(tokens_input + tokens_output) as tokens
FROM usage_logs
WHERE created_at >= date('now', '-30 days')
GROUP BY date(created_at)
ORDER BY date DESC;
```

### Session Management

```sql
-- Create session
INSERT INTO sessions (id, expires_at) 
VALUES (?, datetime('now', '+7 days'));

-- Validate session
SELECT id FROM sessions 
WHERE id = ? AND datetime(expires_at) > datetime('now');

-- Cleanup expired
DELETE FROM sessions WHERE datetime(expires_at) < datetime('now');
```

---

## Backup & Restore

### Backup

```bash
# Simple file copy (when proxy stopped)
cp /data/proxypal.db /backup/proxypal-$(date +%Y%m%d).db

# Online backup
sqlite3 /data/proxypal.db ".backup /backup/proxypal-$(date +%Y%m%d).db"
```

### Restore

```bash
# Stop server first
cp /backup/proxypal-20240115.db /data/proxypal.db
```

---

## Performance Considerations

### Indexes
- Primary keys và foreign keys đã indexed
- Thêm indexes cho các columns hay query

### Cleanup Jobs
```sql
-- Delete old usage logs (> 90 days)
DELETE FROM usage_logs 
WHERE created_at < date('now', '-90 days');

-- Keep daily_usage for historical data
```

### Connection Pooling
```rust
// Use r2d2 for connection pooling
use r2d2_sqlite::SqliteConnectionManager;

let manager = SqliteConnectionManager::file("/data/proxypal.db");
let pool = r2d2::Pool::builder()
    .max_size(10)
    .build(manager)?;
```
