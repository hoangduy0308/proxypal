# ProxyPal Server - Render.com Deployment Guide

Deploy ProxyPal Server to Render.com for shared access to your AI subscriptions.

## Prerequisites

### 1. Render Account
- Create account at [render.com](https://render.com)
- Optional: Install [Render CLI](https://render.com/docs/cli) for local management

### 2. Provider OAuth Setup

Before deploying, register OAuth applications with your AI providers:

| Provider | Developer Console |
|----------|-------------------|
| Claude | [console.anthropic.com](https://console.anthropic.com) |
| OpenAI | [platform.openai.com](https://platform.openai.com) |
| Google Gemini | [console.cloud.google.com](https://console.cloud.google.com) |
| GitHub Copilot | [github.com/settings/developers](https://github.com/settings/developers) |

**Callback URL Format:**
```
https://<your-app-name>.onrender.com/oauth/<provider>/callback
```

Example callback URLs (replace `proxypal-server` with your service name):
- Claude: `https://proxypal-server.onrender.com/oauth/claude/callback`
- ChatGPT: `https://proxypal-server.onrender.com/oauth/chatgpt/callback`
- Gemini: `https://proxypal-server.onrender.com/oauth/gemini/callback`
- Copilot: `https://proxypal-server.onrender.com/oauth/copilot/callback`

---

## Environment Variables

### Required (Must Set Manually)

| Variable | Description | How to Generate |
|----------|-------------|-----------------|
| `ADMIN_PASSWORD` | Initial admin password | Choose a strong password. Only used on first run to create admin account. |
| `ENCRYPTION_KEY` | 32-byte key for token encryption | See below |

**Generate ENCRYPTION_KEY:**
```bash
# Using OpenSSL (hex format - recommended)
openssl rand -hex 32

# Using OpenSSL (base64 format)
openssl rand -base64 32

# Using Python
python -c "import secrets; print(secrets.token_hex(32))"
```

> ⚠️ **WARNING: ENCRYPTION_KEY Rotation**
>
> The `ENCRYPTION_KEY` encrypts all stored OAuth tokens. If you change or lose this key:
> - ALL stored provider tokens become invalid
> - Users must re-authenticate with ALL providers
> - There is NO recovery mechanism
>
> **Store this key securely outside of Render!**

### Automatic (Set by render.yaml)

| Variable | Default | Description |
|----------|---------|-------------|
| `DATABASE_PATH` | `/data/proxypal.db` | SQLite database location |
| `DATA_DIR` | `/data` | Persistent data directory |
| `CLIPROXY_BINARY_PATH` | `/app/cliproxyapi` | CLIProxyAPI binary path |
| `PROXY_MANAGEMENT_URL` | `http://127.0.0.1:8317` | Internal proxy URL |
| `MANAGEMENT_KEY` | `proxypal-mgmt-key` | Internal management key |

---

## Deployment Steps

### 1. Fork/Clone Repository

```bash
# Fork on GitHub, then clone
git clone https://github.com/YOUR_USERNAME/proxypal.git
cd proxypal
```

### 2. Configure OAuth Callback URLs

Register callback URLs with each provider you plan to use (see Prerequisites section).

### 3. Create Render Service

**Option A: Using Render Dashboard**
1. Go to [dashboard.render.com](https://dashboard.render.com)
2. Click **New** → **Blueprint**
3. Connect your GitHub repository
4. Select the `render.yaml` file
5. Click **Apply**

**Option B: Using Render CLI**
```bash
render blueprint apply
```

### 4. Set Secrets in Render Dashboard

1. Go to your service in Render Dashboard
2. Click **Environment** tab
3. Add the following secrets:
   - `ADMIN_PASSWORD`: Your chosen admin password
   - `ENCRYPTION_KEY`: Generated 32-byte hex key

### 5. Deploy

Click **Manual Deploy** → **Deploy latest commit** or push to your main branch.

### 6. Monitor Deploy Logs

Watch the deployment logs for:
- ✅ Docker image build success
- ✅ Health check passing (`/healthz` returns 200)
- ✅ "Admin password initialized" message (first run only)

---

## Post-Deploy Verification

### 1. Health Check
```bash
curl https://YOUR-APP.onrender.com/healthz
# Expected: {"status":"ok","timestamp":"..."}
```

### 2. Admin Login Test
```bash
curl -X POST https://YOUR-APP.onrender.com/api/auth/login \
  -H "Content-Type: application/json" \
  -d '{"password":"YOUR_ADMIN_PASSWORD"}'
# Expected: 200 OK with session cookie
```

### 3. Automated Smoke Test
```bash
./scripts/post-deploy-smoke.sh https://YOUR-APP.onrender.com YOUR_ADMIN_PASSWORD
```

---

## Troubleshooting

### Common Issues

| Issue | Cause | Solution |
|-------|-------|----------|
| 503 Service Unavailable | Service starting or crashed | Check deploy logs, wait 2-3 minutes |
| Health check failing | CLIProxyAPI not starting | Verify Docker build includes binary |
| "ADMIN_PASSWORD required" error | Missing env var | Set ADMIN_PASSWORD in Render dashboard |
| OAuth callback error | Wrong callback URL | Update callback URL in provider console |
| Tokens decryption failed | Wrong/changed ENCRYPTION_KEY | Restore original key or re-authenticate providers |

### Accessing Logs

**Render Dashboard:**
1. Go to your service
2. Click **Logs** tab
3. Filter by severity if needed

**Render CLI:**
```bash
render logs --service proxypal-server --tail
```

### Database Recovery

The database is stored on a Render Disk at `/data/proxypal.db`.

**Backup:**
```bash
# SSH into service (if enabled) or use Render's backup feature
render ssh --service proxypal-server
cp /data/proxypal.db /data/backup-$(date +%Y%m%d).db
```

**Reset Database:**
If you need to start fresh:
1. Delete the service in Render
2. Re-deploy from blueprint
3. Database will be recreated on first run

---

## Security Notes

### Single Instance Requirement

> ⚠️ **CRITICAL: This service MUST run as a single instance**

The `render.yaml` is configured with:
```yaml
scaling:
  minInstances: 1
  maxInstances: 1
```

**Why?** SQLite does not support concurrent writes from multiple processes. Running multiple instances will cause database corruption.

**Need horizontal scaling?** Migrate to PostgreSQL (requires code changes).

### ENCRYPTION_KEY Handling

- Generate once, store permanently
- Never commit to version control
- Store backup in a secure password manager
- Rotation requires all users to re-authenticate

### Session Management

- Sessions are stored in the database
- Sessions survive service restarts
- Sessions expire after 24 hours of inactivity
- Admin can invalidate all sessions by changing ADMIN_PASSWORD

### Network Security

- All traffic uses HTTPS (Render provides TLS)
- Internal proxy communication is localhost-only
- API keys are hashed before storage
- OAuth tokens are encrypted at rest

---

## Updating

### Pull Upstream Changes
```bash
git remote add upstream https://github.com/heyhuynhgiabuu/proxypal.git
git fetch upstream
git merge upstream/main
git push origin main
```

Render will automatically deploy on push to main branch.

### Manual Redeploy
1. Go to Render Dashboard
2. Click **Manual Deploy** → **Deploy latest commit**

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                     Render.com                              │
│  ┌─────────────────────────────────────────────────────┐    │
│  │              ProxyPal Server Container              │    │
│  │                                                     │    │
│  │  ┌─────────────┐      ┌──────────────────────┐     │    │
│  │  │   Axum      │──────│    CLIProxyAPI       │     │    │
│  │  │   Server    │      │    (Port 8317)       │     │    │
│  │  │  (Port 80)  │      └──────────────────────┘     │    │
│  │  └─────────────┘                                   │    │
│  │         │                                          │    │
│  │  ┌──────┴──────┐                                   │    │
│  │  │   SQLite    │◄──────── /data (Render Disk)     │    │
│  │  └─────────────┘                                   │    │
│  └─────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
          │
          ▼
    ┌─────────────┐
    │  AI APIs    │
    │  (Claude,   │
    │   OpenAI,   │
    │   etc.)     │
    └─────────────┘
```

---

## Support

- **Issues**: [GitHub Issues](https://github.com/heyhuynhgiabuu/proxypal/issues)
- **Documentation**: [docs/](../docs/)
