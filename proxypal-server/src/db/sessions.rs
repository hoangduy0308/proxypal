use anyhow::Result;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub csrf_token: String,
    pub expires_at: String,
    pub created_at: String,
    pub last_accessed: String,
}

impl Database {
    pub fn create_session(&self, id: &str, csrf_token: &str, ttl_days: i64) -> Result<Session> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO sessions (id, csrf_token, expires_at, created_at, last_accessed)
                 VALUES (?1, ?2, datetime('now', ?3 || ' days'), datetime('now'), datetime('now'))",
                params![id, csrf_token, ttl_days],
            )?;
            
            let mut stmt = conn.prepare(
                "SELECT id, csrf_token, expires_at, created_at, last_accessed FROM sessions WHERE id = ?1"
            )?;
            let session = stmt.query_row(params![id], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    csrf_token: row.get(1)?,
                    expires_at: row.get(2)?,
                    created_at: row.get(3)?,
                    last_accessed: row.get(4)?,
                })
            })?;
            Ok(session)
        })
    }

    pub fn get_session(&self, id: &str) -> Result<Option<Session>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, csrf_token, expires_at, created_at, last_accessed 
                 FROM sessions 
                 WHERE id = ?1 AND datetime(expires_at) > datetime('now')"
            )?;
            let session = stmt.query_row(params![id], |row| {
                Ok(Session {
                    id: row.get(0)?,
                    csrf_token: row.get(1)?,
                    expires_at: row.get(2)?,
                    created_at: row.get(3)?,
                    last_accessed: row.get(4)?,
                })
            }).optional()?;
            Ok(session)
        })
    }

    pub fn update_session_access(&self, id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE sessions SET last_accessed = datetime('now') WHERE id = ?1",
                params![id],
            )?;
            Ok(())
        })
    }

    pub fn delete_session(&self, id: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute("DELETE FROM sessions WHERE id = ?1", params![id])?;
            Ok(())
        })
    }

    pub fn cleanup_expired_sessions(&self) -> Result<u64> {
        self.with_conn(|conn| {
            let deleted = conn.execute(
                "DELETE FROM sessions WHERE datetime(expires_at) <= datetime('now')",
                [],
            )?;
            Ok(deleted as u64)
        })
    }
}
