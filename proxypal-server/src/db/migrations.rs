use super::Database;
use anyhow::Result;

pub fn run(db: &Database) -> Result<()> {
    db.with_conn(|conn| {
        conn.execute_batch(
            r#"
            PRAGMA foreign_keys = ON;

            -- Users table
            CREATE TABLE IF NOT EXISTS users (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                name            TEXT NOT NULL UNIQUE,
                api_key_hash    TEXT NOT NULL,
                api_key_prefix  TEXT NOT NULL,
                quota_tokens    INTEGER,
                used_tokens     INTEGER NOT NULL DEFAULT 0,
                enabled         INTEGER NOT NULL DEFAULT 1,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                last_used_at    TEXT
            );

            -- Usage logs table
            CREATE TABLE IF NOT EXISTS usage_logs (
                id              INTEGER PRIMARY KEY AUTOINCREMENT,
                user_id         INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                provider        TEXT NOT NULL,
                model           TEXT NOT NULL,
                tokens_input    INTEGER NOT NULL,
                tokens_output   INTEGER NOT NULL,
                request_time_ms INTEGER NOT NULL,
                status          TEXT DEFAULT 'success',
                timestamp       TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- Create index for usage lookups
            CREATE INDEX IF NOT EXISTS idx_usage_user_id ON usage_logs(user_id);
            CREATE INDEX IF NOT EXISTS idx_usage_timestamp ON usage_logs(timestamp);

            -- Settings table (key-value store)
            CREATE TABLE IF NOT EXISTS settings (
                key   TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            -- Sessions table (admin login sessions)
            CREATE TABLE IF NOT EXISTS sessions (
                id              TEXT PRIMARY KEY,
                csrf_token      TEXT NOT NULL,
                expires_at      TEXT NOT NULL,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                last_accessed   TEXT NOT NULL DEFAULT (datetime('now'))
            );

            CREATE INDEX IF NOT EXISTS idx_sessions_expires_at ON sessions(expires_at);

            -- Provider accounts table (for OAuth tokens)
            CREATE TABLE IF NOT EXISTS provider_accounts (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                provider    TEXT NOT NULL,
                account_id  TEXT NOT NULL,
                tokens      TEXT NOT NULL,  -- encrypted JSON
                enabled     INTEGER NOT NULL DEFAULT 1,
                created_at  TEXT NOT NULL DEFAULT (datetime('now')),
                UNIQUE(provider, account_id)
            );

            -- Providers table
            CREATE TABLE IF NOT EXISTS providers (
                id          INTEGER PRIMARY KEY AUTOINCREMENT,
                name        TEXT NOT NULL UNIQUE,
                type        TEXT NOT NULL,  -- 'oauth' or 'api_key'
                enabled     INTEGER NOT NULL DEFAULT 1,
                settings    TEXT NOT NULL DEFAULT '{}',
                created_at  TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at  TEXT NOT NULL DEFAULT (datetime('now'))
            );

            -- OAuth states table (for CSRF protection during OAuth flow)
            CREATE TABLE IF NOT EXISTS oauth_states (
                state           TEXT PRIMARY KEY,
                provider        TEXT NOT NULL,
                admin_session_id TEXT NOT NULL,
                redirect_url    TEXT,
                created_at      TEXT NOT NULL DEFAULT (datetime('now')),
                expires_at      TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_oauth_states_expires_at ON oauth_states(expires_at);
            "#,
        )?;
        Ok(())
    })
}
