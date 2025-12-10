# ProxyPal Server - Implementation Plan

## Project Timeline

**Total Estimated Effort:** 3-4 tuần  
**Target Deployment:** Render.com  

---

## Phase 1: Foundation (Week 1)

### 1.1 Project Structure Setup (Day 1)

**Tasks:**
- [ ] Tạo Cargo workspace configuration
- [ ] Tạo `proxypal-server` crate
- [ ] Setup dependencies (Axum, Tower, SQLite, etc.)
- [ ] Tạo `core` module structure

**Files to create:**
```
Cargo.toml                          # Workspace root
proxypal-server/
├── Cargo.toml
└── src/
    └── main.rs

src-tauri/src/core/
├── mod.rs
├── config.rs
├── types.rs
└── proxy.rs
```

**Commands:**
```bash
# Test workspace builds
cargo build --workspace
```

### 1.2 Extract Core Module (Day 2-3)

**Tasks:**
- [ ] Extract shared types from `lib.rs` to `core/types.rs`
  - `ProxyStatus`
  - `AuthStatus`
  - `AppConfig`
  - `Provider` types
  - `RequestLog`
- [ ] Extract config logic to `core/config.rs`
  - YAML generation functions
  - Config file read/write
- [ ] Extract proxy management to `core/proxy.rs`
  - Spawn CLIProxyAPI
  - Health check
  - Stop/restart

**Validation:**
- [ ] Desktop app still compiles and works
- [ ] Core module compiles independently

### 1.3 Basic HTTP Server (Day 3-4)

**Tasks:**
- [ ] Setup Axum server with basic routes
- [ ] Health check endpoint: `GET /healthz`
- [ ] Static file serving for frontend
- [ ] Environment configuration

**Code Structure:**
```rust
// proxypal-server/src/main.rs
#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/healthz", get(health_check))
        .nest_service("/", ServeDir::new("dist"))
        .with_state(app_state);
    
    let port = std::env::var("PORT").unwrap_or("3000".to_string());
    axum::serve(listener, app).await.unwrap();
}
```

### 1.4 Database Setup (Day 4-5)

**Tasks:**
- [ ] Setup SQLite with rusqlite
- [ ] Create migration system
- [ ] Implement tables: `users`, `usage_logs`, `settings`
- [ ] Connection pool setup

**Files:**
```
proxypal-server/src/
├── db/
│   ├── mod.rs
│   ├── migrations.rs
│   ├── users.rs
│   └── usage.rs
```

---

## Phase 2: Core Features (Week 2)

### 2.1 Admin Authentication (Day 1)

**Tasks:**
- [ ] Password-based admin auth
- [ ] Session management (cookie-based)
- [ ] Auth middleware for protected routes
- [ ] Login/logout endpoints
- [ ] CSRF protection

**Endpoints:**
```
POST /api/auth/login     { password: string }
POST /api/auth/logout
GET  /api/auth/status
```

**Security:**
- Password hashed with Argon2
- Session token in HttpOnly cookie
- CSRF protection

**CSRF Implementation:**
```
Header: X-CSRF-Token
Cookie: csrf_token (NOT HttpOnly, SameSite=Strict)

Flow:
1. On login success, server sets csrf_token cookie
2. Frontend reads cookie, sends X-CSRF-Token header on all mutating requests
3. Server validates header matches cookie

Routes requiring CSRF:
- All POST/PUT/DELETE under /api/* (except /api/auth/login)
```

**Admin Password Bootstrap:**
```rust
// On server startup:
async fn bootstrap_admin_password(db: &Database) -> Result<()> {
    let existing = db.get_setting("admin_password_hash").await?;
    
    if existing.is_none() {
        // First run: hash password from env var
        let password = std::env::var("ADMIN_PASSWORD")
            .expect("ADMIN_PASSWORD env var required on first run");
        let hash = argon2_hash(&password)?;
        db.set_setting("admin_password_hash", &hash).await?;
        info!("Admin password initialized from ADMIN_PASSWORD env var");
    }
    // Subsequent runs: ignore ADMIN_PASSWORD, use stored hash
    Ok(())
}
```

### 2.2 User Management API (Day 2-3)

**Tasks:**
- [ ] CRUD endpoints for users
- [ ] API key generation
- [ ] Quota management
- [ ] User enable/disable

**Endpoints:**
```
GET    /api/users              # List all users
POST   /api/users              # Create user
GET    /api/users/:id          # Get user details
PUT    /api/users/:id          # Update user
DELETE /api/users/:id          # Delete user
POST   /api/users/:id/regenerate-key  # New API key
```

**User Model:**
```rust
struct User {
    id: i64,
    name: String,
    api_key_hash: String,
    api_key_prefix: String,     // "sk-john" for display
    quota_tokens: Option<i64>,  // None = unlimited
    used_tokens: i64,
    enabled: bool,
    created_at: DateTime<Utc>,
    last_used_at: Option<DateTime<Utc>>,
}
```

### 2.3 Usage Tracking (Day 3-4)

**Tasks:**
- [ ] Middleware to intercept proxy responses
- [ ] Parse token counts from responses
- [ ] Log usage per user/provider
- [ ] Usage statistics API

**Endpoints:**
```
GET /api/usage                  # Overall stats
GET /api/usage/users/:id        # Per-user stats
GET /api/usage/providers        # Per-provider stats
GET /api/usage/daily            # Daily breakdown
```

**Usage Log Model:**
```rust
struct UsageLog {
    id: i64,
    user_id: i64,
    provider: String,           // "claude", "openai", "gemini"
    model: String,              // "claude-3-opus", "gpt-4"
    tokens_input: i64,
    tokens_output: i64,
    request_time_ms: i64,
    timestamp: DateTime<Utc>,
}
```

### 2.4 Proxy API with Auth (Day 4-5)

**Tasks:**
- [ ] API key validation middleware
- [ ] Route requests through CLIProxyAPI
- [ ] Response interception for usage logging
- [ ] Rate limiting per user

**Flow:**
```
Request → API Key Middleware → Quota Check → CLIProxyAPI → Usage Log → Response
```

**Middleware Logic:**
```rust
async fn api_key_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    let api_key = extract_api_key(&headers)?;
    let user = validate_api_key(&state.db, &api_key).await?;
    check_quota(&user)?;
    
    // Forward to CLIProxyAPI
    let response = forward_to_proxy(request).await?;
    
    // Log usage
    log_usage(&state.db, &user, &response).await?;
    
    Ok(response)
}
```

---

## Phase 3: OAuth & Providers (Week 3)

### 3.1 OAuth Flow Adaptation (Day 1-2)

**Tasks:**
- [ ] OAuth start endpoint per provider
- [ ] Callback handler
- [ ] Token storage (encrypted)
- [ ] Session state management

**Endpoints:**
```
GET /oauth/:provider/start      # Redirect to provider
GET /oauth/:provider/callback   # Handle callback
```

**Providers to support:**
- Claude (Anthropic)
- ChatGPT (OpenAI)
- Gemini (Google)
- Copilot (GitHub)

**Token Encryption:**
```rust
// Encrypt with AES-256-GCM before storing
let encrypted = encrypt_token(&token, &env::var("ENCRYPTION_KEY")?)?;
db.store_provider_token(provider, encrypted).await?;
```

### 3.2 Provider Management API (Day 2-3)

**Tasks:**
- [ ] List configured providers
- [ ] Provider status/health
- [ ] Remove provider
- [ ] Provider settings

**Endpoints:**
```
GET    /api/providers           # List all providers
GET    /api/providers/:name     # Provider details
DELETE /api/providers/:name     # Remove provider
PUT    /api/providers/:name     # Update settings
GET    /api/providers/:name/health  # Check health
```

### 3.3 Proxy Configuration (Day 3-4)

**Tasks:**
- [ ] Generate proxy-config.yaml from database
- [ ] Hot-reload CLIProxyAPI on config change
- [ ] Load balancing settings
- [ ] Model mappings

**Config Generation:**
```rust
fn generate_proxy_config(providers: Vec<Provider>) -> String {
    // Generate YAML matching CLIProxyAPI format
    // Include all authenticated accounts
    // Apply load balancing settings
}
```

### 3.4 Migrate Existing Tauri Commands (Day 4-5)

**Tasks:**
- [ ] Map remaining Tauri commands to HTTP endpoints
- [ ] Proxy status/start/stop
- [ ] Logs streaming (WebSocket)
- [ ] Settings management

**Command Mapping:**
| Tauri Command | HTTP Endpoint |
|---------------|---------------|
| `get_proxy_status` | `GET /api/proxy/status` |
| `start_proxy` | `POST /api/proxy/start` |
| `stop_proxy` | `POST /api/proxy/stop` |
| `get_config` | `GET /api/config` |
| `save_config` | `PUT /api/config` |
| `get_request_logs` | `GET /api/logs` |

---

## Phase 4: Frontend Adaptation (Week 3-4)

### 4.1 Backend Adapter Layer (Day 1-2)

**Tasks:**
- [ ] Create backend adapter interface
- [ ] Implement HTTP client
- [ ] Environment-based client selection
- [ ] Error handling wrapper

**Files:**
```
src/backend/
├── index.ts           # Export unified interface
├── types.ts           # Shared types
├── tauriClient.ts     # Tauri implementation
└── httpClient.ts      # HTTP implementation
```

**Adapter Interface:**
```typescript
// src/backend/types.ts
export interface BackendClient {
  // Proxy
  getProxyStatus(): Promise<ProxyStatus>;
  startProxy(): Promise<void>;
  stopProxy(): Promise<void>;
  
  // Users
  listUsers(): Promise<User[]>;
  createUser(name: string): Promise<User>;
  deleteUser(id: number): Promise<void>;
  regenerateApiKey(id: number): Promise<string>;
  
  // Providers
  listProviders(): Promise<Provider[]>;
  startOAuth(provider: string): Promise<void>;
  removeProvider(name: string): Promise<void>;
  
  // Usage
  getUsageStats(): Promise<UsageStats>;
  getUserUsage(id: number): Promise<UserUsage>;
  
  // Config
  getConfig(): Promise<AppConfig>;
  saveConfig(config: AppConfig): Promise<void>;
}
```

**HTTP Client:**
```typescript
// src/backend/httpClient.ts
export const httpClient: BackendClient = {
  async getProxyStatus() {
    const res = await fetch('/api/proxy/status');
    return res.json();
  },
  
  async startProxy() {
    await fetch('/api/proxy/start', { method: 'POST' });
  },
  
  // ... more implementations
};
```

### 4.2 Update UI Components (Day 2-4)

**Tasks:**
- [ ] Replace `invoke()` calls with backend adapter
- [ ] Add User Management page
- [ ] Add Usage Dashboard
- [ ] Update Provider management for OAuth flow

**Pages to update:**
- `Dashboard.tsx` - Proxy status, quick stats
- `Providers.tsx` - OAuth flow changes
- `Settings.tsx` - Config management
- `Users.tsx` (NEW) - User management
- `Usage.tsx` (NEW) - Usage statistics

### 4.3 Build & Embed (Day 4-5)

**Tasks:**
- [ ] Configure Vite for production build
- [ ] Embed static files in Rust binary
- [ ] Test static file serving
- [ ] Environment variable injection

**Build Process:**
```bash
# Frontend build
cd proxypal
pnpm build

# Server build with embedded frontend
cd proxypal-server
cargo build --release
```

**Embedding Options:**
1. **Runtime serving**: Serve from `dist/` directory
2. **Compile-time embedding**: Use `rust-embed` crate

---

## Phase 5: Deployment (Week 4)

### 5.1 Docker Setup (Day 1-2)

**Tasks:**
- [ ] Create Dockerfile
- [ ] Multi-stage build (Rust + Node)
- [ ] Include CLIProxyAPI binary
- [ ] Environment configuration

**Dockerfile:**
```dockerfile
# Stage 1: Build frontend
FROM node:20-alpine AS frontend
WORKDIR /app
COPY package.json pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install
COPY . .
RUN pnpm build

# Stage 2: Build Rust
FROM rust:1.75 AS backend
WORKDIR /app
COPY . .
COPY --from=frontend /app/dist ./dist
RUN cargo build --release -p proxypal-server

# Stage 3: Runtime
FROM debian:bookworm-slim
WORKDIR /app
COPY --from=backend /app/target/release/proxypal-server .
COPY --from=backend /app/cliproxyapi .
COPY --from=frontend /app/dist ./dist

ENV PORT=3000
EXPOSE 3000
CMD ["./proxypal-server"]
```

### 5.2 Render Configuration (Day 2-3)

**Tasks:**
- [ ] Create `render.yaml`
- [ ] Configure Render Disk for persistence
- [ ] Set environment variables
- [ ] Configure health checks

> ⚠️ **IMPORTANT**: Set `scale: 1` - SQLite + Render Disk không hỗ trợ multiple instances!

**render.yaml:**
```yaml
services:
  - type: web
    name: proxypal-server
    runtime: docker
    dockerfilePath: ./Dockerfile
    scaling:
      minInstances: 1
      maxInstances: 1    # MUST be 1 for SQLite
    envVars:
      - key: ADMIN_PASSWORD
        sync: false
      - key: ENCRYPTION_KEY
        generateValue: true
      - key: DATABASE_PATH
        value: /data/proxypal.db
    disk:
      name: proxypal-data
      mountPath: /data
      sizeGB: 1
    healthCheckPath: /healthz
```

> ⚠️ **ENCRYPTION_KEY Warning**: Rotating this key will invalidate ALL stored provider tokens. Users will need to re-authenticate with all providers.

### 5.3 Testing & Documentation (Day 3-4)

**Tasks:**
- [ ] End-to-end testing
- [ ] Load testing (5 users)
- [ ] Documentation updates
- [ ] Deployment guide

**Test Scenarios:**
1. Admin login and provider setup
2. User creation and API key generation
3. Proxy requests with usage tracking
4. OAuth flow for all providers
5. Quota enforcement
6. Server restart/recovery

### 5.4 Launch & Monitoring (Day 4-5)

**Tasks:**
- [ ] Deploy to Render
- [ ] Verify all functionality
- [ ] Setup monitoring/alerts
- [ ] Share with users

---

## Risk Mitigation

### Potential Issues & Solutions

| Risk | Mitigation |
|------|------------|
| OAuth callback URL mismatch | Register Render URL with providers before deployment |
| CLIProxyAPI won't start on Linux | Test with Docker locally first |
| Session persistence across deploys | Use Render Disk for session store |
| Provider tokens lost on redeploy | Encrypt and store in persistent SQLite |
| Usage tracking inaccurate | Add retry logic, fallback to request counting |

### Fallback Options

1. **If OAuth too complex**: Start with API key input only
2. **If usage tracking fails**: Log requests only, skip token counting
3. **If Docker issues**: Try native Rust build on Render
4. **If persistence issues**: Use external SQLite (Turso) or PostgreSQL

---

## Success Criteria

- [ ] Admin can login to dashboard
- [ ] Admin can add Claude/ChatGPT via OAuth
- [ ] Admin can create users with API keys
- [ ] End users can use proxy with their API key
- [ ] Usage tracked per user
- [ ] System survives restart/redeploy
- [ ] Can merge upstream ProxyPal updates

---

## Post-Launch Improvements

### Phase 6 (Future)

- [ ] Real-time usage dashboard (WebSocket)
- [ ] Email notifications for quota limits
- [ ] Multiple admin accounts
- [ ] Provider-specific quotas
- [ ] Cost estimation per user
- [ ] API rate limiting improvements
- [ ] Audit logging
- [ ] Backup/restore functionality
