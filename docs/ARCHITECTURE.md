# ProxyPal Server - Architecture Document

## Overview

ProxyPal Server là phiên bản cloud-deployable của ProxyPal desktop app, cho phép deploy lên Render.com hoặc các cloud platform khác để cung cấp API proxy cho nhiều người dùng.

## Use Cases

```
┌─────────────────────────────────────────────────────────────────┐
│                         ADMIN                                    │
│  - Quản lý AI providers (Claude, ChatGPT, Gemini, Copilot)      │
│  - Tạo/xóa users và API keys                                    │
│  - Monitor usage per user                                        │
│  - Set quotas/limits                                             │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    PROXYPAL SERVER                               │
│  ┌─────────────────────┐    ┌─────────────────────────────────┐ │
│  │  Admin Dashboard    │    │  Proxy API (CLIProxyAPI)        │ │
│  │  - Port 3000        │    │  - Port 8317                    │ │
│  │  - Password auth    │    │  - API key auth per user        │ │
│  │  - SolidJS UI       │    │  - OpenAI-compatible            │ │
│  └─────────────────────┘    └─────────────────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                         END USERS                                │
│  - Cursor IDE: OPENAI_API_KEY=sk-user-xxx                       │
│  - Cline: API endpoint = https://your-app.onrender.com          │
│  - Any OpenAI-compatible client                                  │
└─────────────────────────────────────────────────────────────────┘
```

## High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    ProxyPal Server Binary                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Axum HTTP Server                       │   │
│  │                                                           │   │
│  │  ┌─────────────────┐  ┌─────────────────┐                │   │
│  │  │  Static Files   │  │  REST API       │                │   │
│  │  │  (SolidJS dist) │  │  /api/*         │                │   │
│  │  └─────────────────┘  └─────────────────┘                │   │
│  │                                                           │   │
│  │  ┌─────────────────┐  ┌─────────────────┐                │   │
│  │  │  OAuth Handler  │  │  Admin Auth     │                │   │
│  │  │  /oauth/*       │  │  Middleware     │                │   │
│  │  └─────────────────┘  └─────────────────┘                │   │
│  │                                                           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    Core Module                            │   │
│  │                                                           │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐ │   │
│  │  │ Config      │ │ Types       │ │ Proxy Manager       │ │   │
│  │  │ Management  │ │ & Models    │ │ (spawn CLIProxyAPI) │ │   │
│  │  └─────────────┘ └─────────────┘ └─────────────────────┘ │   │
│  │                                                           │   │
│  │  ┌─────────────┐ ┌─────────────┐ ┌─────────────────────┐ │   │
│  │  │ User        │ │ Usage       │ │ OAuth               │ │   │
│  │  │ Management  │ │ Tracking    │ │ Handler             │ │   │
│  │  └─────────────┘ └─────────────┘ └─────────────────────┘ │   │
│  │                                                           │   │
│  └──────────────────────────────────────────────────────────┘   │
│                              │                                   │
│                              ▼                                   │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    CLIProxyAPI (Sidecar)                  │   │
│  │                    - Port 8317                            │   │
│  │                    - Handles actual AI API proxying       │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                    SQLite Database                        │   │
│  │                    - Users, API keys, usage logs          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Component Details

### 1. Axum HTTP Server

**Responsibilities:**
- Serve SolidJS frontend static files
- Handle REST API requests for admin operations
- OAuth callback handling
- Admin authentication middleware

**Technology:** Rust + Axum + Tower

### 2. Core Module (Shared Code)

Extracted from current `src-tauri/src/lib.rs` to be shared between desktop and server:

```
src-tauri/src/core/
├── mod.rs
├── config.rs      # AppConfig, YAML generation
├── types.rs       # ProxyStatus, AuthStatus, Provider types
├── proxy.rs       # Start/stop CLIProxyAPI
├── users.rs       # User CRUD, API key generation
├── usage.rs       # Usage tracking, statistics
└── oauth.rs       # OAuth flow handling
```

### 3. CLIProxyAPI Sidecar

- Spawned as child process by server
- Handles actual AI API proxying
- Configured via `proxy-config.yaml`
- Logs requests for usage tracking

> ⚠️ **SECURITY**: CLIProxyAPI **MUST bind to `127.0.0.1:8317` only**, not `0.0.0.0`. Only Axum server may call it. If exposed publicly, attackers can bypass API key auth and use provider tokens directly.

### 4. SQLite Database

Persistent storage for:
- User accounts and API keys
- Usage statistics per user
- Configuration data

### 5. Frontend (SolidJS)

Modified to support both Tauri and HTTP backends:

```
src/backend/
├── index.ts           # Backend adapter interface
├── tauriClient.ts     # Tauri invoke() implementation
└── httpClient.ts      # HTTP fetch() implementation
```

## Provider Token Storage

> **Source of Truth**: Provider tokens are stored **only in SQLite** (`provider_accounts.tokens` - encrypted).
> 
> `proxy-config.yaml` is **generated** from database on each config change, not a primary source.
> `/data/auth/` directory is used by CLIProxyAPI internally but managed by the server.

```
Database (primary)     →    proxy-config.yaml (generated)    →    CLIProxyAPI
provider_accounts.tokens    Generated on startup/change           Reads config
(encrypted)                 Regenerated on provider add/remove    Handles requests
```

---

## Data Flow

### Request Flow (End User → AI Provider)

```
End User (Cursor/Cline)
    │
    │ POST /v1/chat/completions
    │ Authorization: Bearer sk-user-xxx
    │
    ▼
┌─────────────────────────┐
│ Axum Server             │
│ (API Key Middleware)    │
│ - Validate API key      │
│ - Identify user         │
│ - Check quota           │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ CLIProxyAPI             │
│ - Route to provider     │
│ - Use provider token    │
│ - Return response       │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ Usage Tracker           │
│ - Parse response tokens │
│ - Log to database       │
│ - Update user quota     │
└─────────────────────────┘
```

### Admin OAuth Flow

```
Admin clicks "Add Claude Account"
    │
    ▼
┌─────────────────────────┐
│ GET /oauth/claude/start │
│ - Generate state token  │
│ - Store in session      │
│ - Redirect to Claude    │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────┐
│ Claude OAuth Server     │
│ - User authenticates    │
│ - Authorize ProxyPal    │
└───────────┬─────────────┘
            │
            ▼
┌─────────────────────────────────────────┐
│ GET /oauth/claude/callback?code=xxx    │
│ - Validate state                        │
│ - Exchange code for tokens              │
│ - Store encrypted tokens                │
│ - Update proxy-config.yaml              │
│ - Restart CLIProxyAPI                   │
└─────────────────────────────────────────┘
```

## Security Considerations

### Admin Authentication
- Password-based or token-based
- Session stored in secure cookie
- Rate limiting on login attempts

### User API Keys
- Format: `sk-{username}-{random32chars}`
- Stored hashed in database
- Can be regenerated/revoked

### Provider Tokens
- Encrypted at rest (AES-256-GCM)
- Encryption key from environment variable
- Never exposed to end users

### Network Security
- HTTPS required in production
- CORS configured for admin dashboard only
- Rate limiting per API key

## Deployment Architecture (Render.com)

```
┌─────────────────────────────────────────────────────────────────┐
│                         Render.com                               │
│                                                                  │
│  ┌───────────────────────────────────────────────────────────┐  │
│  │                    Web Service                             │  │
│  │                                                            │  │
│  │  ┌─────────────────────────────────────────────────────┐  │  │
│  │  │  proxypal-server binary                             │  │  │
│  │  │  - Listens on $PORT (Render provides)               │  │  │
│  │  │  - Spawns CLIProxyAPI internally                    │  │  │
│  │  └─────────────────────────────────────────────────────┘  │  │
│  │                                                            │  │
│  │  Environment Variables:                                    │  │
│  │  - ADMIN_PASSWORD                                          │  │
│  │  - ENCRYPTION_KEY                                          │  │
│  │  - DATABASE_PATH                                           │  │
│  │                                                            │  │
│  └───────────────────────────────────────────────────────────┘  │
│                              │                                   │
│  ┌───────────────────────────┴───────────────────────────────┐  │
│  │                    Render Disk                             │  │
│  │  - /data/proxypal.db (SQLite)                             │  │
│  │  - /data/proxy-config.yaml                                │  │
│  │  - /data/auth/ (provider tokens)                          │  │
│  └───────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

Public URLs:
- https://proxypal-xxx.onrender.com (Admin Dashboard)
- https://proxypal-xxx.onrender.com/v1/* (Proxy API)
```

## Differences from Desktop Version

| Aspect | Desktop (Tauri) | Server (Axum) |
|--------|-----------------|---------------|
| Process | GUI app | Headless binary |
| API | Tauri invoke() | HTTP REST |
| OAuth | Deep links `proxypal://` | HTTP callbacks |
| Auth | None (local) | Admin password |
| Users | Single | Multi-user with API keys |
| Storage | Local files | SQLite + Render Disk |
| Usage | Per-provider stats | Per-user + per-provider |

## Maintainability Strategy

### Code Structure for Easy Upstream Sync

```
proxypal/                        # Fork of upstream
├── src-tauri/
│   ├── src/
│   │   ├── lib.rs              # ✅ Minimal changes
│   │   └── core/               # ✨ NEW - Extracted shared code
│   └── Cargo.toml              # ✅ Workspace member
│
├── proxypal-server/            # ✨ NEW - Server binary
│   ├── src/
│   │   ├── main.rs
│   │   ├── routes/
│   │   ├── middleware/
│   │   └── handlers/
│   └── Cargo.toml
│
├── src/                        # SolidJS frontend
│   ├── backend/                # ✨ NEW - Backend adapter
│   └── ...                     # ✅ Minimal changes
│
└── Cargo.toml                  # ✨ Workspace root
```

### Merge Strategy

1. **Upstream updates**: `git pull upstream main`
2. **Conflicts minimal**: Most changes in new directories
3. **Shared code**: `core/` module synced manually when upstream changes types
4. **Frontend**: Adapter layer isolates changes

## Technology Stack

| Component | Technology |
|-----------|------------|
| HTTP Server | Rust + Axum + Tower |
| Database | SQLite + rusqlite |
| Frontend | SolidJS + Tailwind |
| Build | Cargo + Vite |
| Deployment | Render.com (Docker or native) |
| Proxy Engine | CLIProxyAPI (Go binary) |
