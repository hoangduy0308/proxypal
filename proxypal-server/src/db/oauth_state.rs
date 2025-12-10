use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthState {
    pub state: String,
    pub provider: String,
    pub admin_session_id: String,
    pub redirect_url: Option<String>,
    pub created_at: String,
    pub expires_at: String,
}

impl Database {
    /// Create a new OAuth state. Returns the state token.
    /// State expires after `ttl_minutes` (default 15).
    pub fn create_oauth_state(
        &self,
        provider: &str,
        admin_session_id: &str,
        redirect_url: Option<&str>,
        ttl_minutes: Option<i64>,
    ) -> Result<String> {
        let state = uuid::Uuid::new_v4().to_string();
        let ttl = ttl_minutes.unwrap_or(15);
        
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO oauth_states (state, provider, admin_session_id, redirect_url, created_at, expires_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now', ?5 || ' minutes'))",
                params![state, provider, admin_session_id, redirect_url, ttl],
            )?;
            Ok(state)
        })
    }

    /// Consume (retrieve and delete) an OAuth state.
    /// Returns None if state doesn't exist or is expired.
    pub fn consume_oauth_state(&self, state: &str) -> Result<Option<OAuthState>> {
        self.with_conn(|conn| {
            let oauth_state: Option<OAuthState> = conn.query_row(
                "SELECT state, provider, admin_session_id, redirect_url, created_at, expires_at
                 FROM oauth_states
                 WHERE state = ?1 AND datetime(expires_at) > datetime('now')",
                params![state],
                |row| {
                    Ok(OAuthState {
                        state: row.get(0)?,
                        provider: row.get(1)?,
                        admin_session_id: row.get(2)?,
                        redirect_url: row.get(3)?,
                        created_at: row.get(4)?,
                        expires_at: row.get(5)?,
                    })
                },
            ).optional()?;

            if oauth_state.is_some() {
                conn.execute("DELETE FROM oauth_states WHERE state = ?1", params![state])?;
            }

            Ok(oauth_state)
        })
    }

    /// Get OAuth state without consuming it (for debugging/admin)
    pub fn get_oauth_state(&self, state: &str) -> Result<Option<OAuthState>> {
        self.with_conn(|conn| {
            let oauth_state = conn.query_row(
                "SELECT state, provider, admin_session_id, redirect_url, created_at, expires_at
                 FROM oauth_states
                 WHERE state = ?1 AND datetime(expires_at) > datetime('now')",
                params![state],
                |row| {
                    Ok(OAuthState {
                        state: row.get(0)?,
                        provider: row.get(1)?,
                        admin_session_id: row.get(2)?,
                        redirect_url: row.get(3)?,
                        created_at: row.get(4)?,
                        expires_at: row.get(5)?,
                    })
                },
            ).optional()?;
            Ok(oauth_state)
        })
    }

    /// Cleanup expired states
    pub fn cleanup_expired_oauth_states(&self) -> Result<u64> {
        self.with_conn(|conn| {
            let deleted = conn.execute(
                "DELETE FROM oauth_states WHERE datetime(expires_at) <= datetime('now')",
                [],
            )?;
            Ok(deleted as u64)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_state_persists_with_provider_and_session() {
        let db = Database::new_in_memory().unwrap();
        
        let state = db.create_oauth_state(
            "github",
            "session-123",
            Some("http://localhost/callback"),
            Some(15),
        ).unwrap();
        
        assert!(!state.is_empty());
        
        let retrieved = db.get_oauth_state(&state).unwrap();
        assert!(retrieved.is_some());
        
        let oauth_state = retrieved.unwrap();
        assert_eq!(oauth_state.state, state);
        assert_eq!(oauth_state.provider, "github");
        assert_eq!(oauth_state.admin_session_id, "session-123");
        assert_eq!(oauth_state.redirect_url, Some("http://localhost/callback".to_string()));
    }

    #[test]
    fn consume_state_returns_and_deletes() {
        let db = Database::new_in_memory().unwrap();
        
        let state = db.create_oauth_state("google", "session-456", None, Some(15)).unwrap();
        
        let first_consume = db.consume_oauth_state(&state).unwrap();
        assert!(first_consume.is_some());
        assert_eq!(first_consume.unwrap().provider, "google");
        
        let second_consume = db.consume_oauth_state(&state).unwrap();
        assert!(second_consume.is_none());
    }

    #[test]
    fn expired_state_is_rejected() {
        let db = Database::new_in_memory().unwrap();
        
        let state = db.create_oauth_state("github", "session-789", None, Some(-1)).unwrap();
        
        let result = db.consume_oauth_state(&state).unwrap();
        assert!(result.is_none());
        
        let get_result = db.get_oauth_state(&state).unwrap();
        assert!(get_result.is_none());
    }

    #[test]
    fn unknown_state_returns_none() {
        let db = Database::new_in_memory().unwrap();
        
        let result = db.consume_oauth_state("nonexistent-state").unwrap();
        assert!(result.is_none());
    }

    #[test]
    fn cleanup_expired_states_removes_old() {
        let db = Database::new_in_memory().unwrap();
        
        db.create_oauth_state("github", "session-1", None, Some(-1)).unwrap();
        db.create_oauth_state("google", "session-2", None, Some(-1)).unwrap();
        let valid_state = db.create_oauth_state("azure", "session-3", None, Some(15)).unwrap();
        
        let deleted = db.cleanup_expired_oauth_states().unwrap();
        assert_eq!(deleted, 2);
        
        let valid = db.get_oauth_state(&valid_state).unwrap();
        assert!(valid.is_some());
    }
}
