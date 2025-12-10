use anyhow::{anyhow, Result};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};
use rand::Rng;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::Database;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: i64,
    pub name: String,
    pub api_key_prefix: String,
    pub quota_tokens: Option<i64>,
    pub used_tokens: i64,
    pub enabled: bool,
    pub created_at: String,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct UserWithHash {
    pub user: User,
    pub api_key_hash: String,
}

fn generate_api_key(name: &str) -> (String, String, String) {
    let random_bytes: [u8; 16] = rand::thread_rng().gen();
    let random_hex = hex::encode(random_bytes);
    let prefix = format!("sk-{}", name);
    let full_key = format!("{}-{}", prefix, random_hex);

    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(full_key.as_bytes(), &salt)
        .expect("Failed to hash API key")
        .to_string();

    (full_key, prefix, hash)
}

fn row_to_user(row: &rusqlite::Row) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        name: row.get(1)?,
        api_key_prefix: row.get(2)?,
        quota_tokens: row.get(3)?,
        used_tokens: row.get(4)?,
        enabled: row.get::<_, i32>(5)? != 0,
        created_at: row.get(6)?,
        last_used_at: row.get(7)?,
    })
}

impl Database {
    pub fn list_users(&self) -> Result<Vec<User>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at FROM users",
            )?;
            let users = stmt
                .query_map([], |row| row_to_user(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(users)
        })
    }

    pub fn create_user(&self, name: &str, quota_tokens: Option<i64>) -> Result<(User, String)> {
        let (full_key, prefix, hash) = generate_api_key(name);

        self.with_conn(|conn| {
            let result = conn.execute(
                "INSERT INTO users (name, api_key_prefix, api_key_hash, quota_tokens, used_tokens, enabled, created_at)
                 VALUES (?1, ?2, ?3, ?4, 0, 1, datetime('now'))",
                rusqlite::params![name, prefix, hash, quota_tokens],
            );

            match result {
                Ok(_) => {
                    let id = conn.last_insert_rowid();
                    let mut stmt = conn.prepare(
                        "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at FROM users WHERE id = ?1",
                    )?;
                    let user = stmt.query_row([id], |row| row_to_user(row))?;
                    Ok((user, full_key))
                }
                Err(rusqlite::Error::SqliteFailure(err, _))
                    if err.code == rusqlite::ErrorCode::ConstraintViolation =>
                {
                    Err(anyhow!("User with name '{}' already exists", name))
                }
                Err(e) => Err(e.into()),
            }
        })
    }

    pub fn get_user_by_id(&self, id: i64) -> Result<Option<User>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at FROM users WHERE id = ?1",
            )?;
            let user = stmt.query_row([id], |row| row_to_user(row)).optional()?;
            Ok(user)
        })
    }

    pub fn get_user_by_api_key_prefix(&self, prefix: &str) -> Result<Option<UserWithHash>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at, api_key_hash 
                 FROM users WHERE api_key_prefix = ?1",
            )?;
            let result = stmt
                .query_row([prefix], |row| {
                    Ok(UserWithHash {
                        user: User {
                            id: row.get(0)?,
                            name: row.get(1)?,
                            api_key_prefix: row.get(2)?,
                            quota_tokens: row.get(3)?,
                            used_tokens: row.get(4)?,
                            enabled: row.get::<_, i32>(5)? != 0,
                            created_at: row.get(6)?,
                            last_used_at: row.get(7)?,
                        },
                        api_key_hash: row.get(8)?,
                    })
                })
                .optional()?;
            Ok(result)
        })
    }

    pub fn update_user(
        &self,
        id: i64,
        name: Option<&str>,
        quota_tokens: Option<Option<i64>>,
        enabled: Option<bool>,
    ) -> Result<Option<User>> {
        self.with_conn(|conn| {
            let mut updates = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(n) = name {
                updates.push("name = ?");
                params.push(Box::new(n.to_string()));
            }
            if let Some(qt) = quota_tokens {
                updates.push("quota_tokens = ?");
                params.push(Box::new(qt));
            }
            if let Some(e) = enabled {
                updates.push("enabled = ?");
                params.push(Box::new(e as i32));
            }

            if updates.is_empty() {
                return self.get_user_by_id(id);
            }

            params.push(Box::new(id));
            let sql = format!(
                "UPDATE users SET {} WHERE id = ?",
                updates.join(", ")
            );

            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let rows_affected = conn.execute(&sql, params_ref.as_slice())?;

            if rows_affected == 0 {
                return Ok(None);
            }

            let mut stmt = conn.prepare(
                "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at FROM users WHERE id = ?1",
            )?;
            let user = stmt.query_row([id], |row| row_to_user(row)).optional()?;
            Ok(user)
        })
    }

    pub fn delete_user(&self, id: i64) -> Result<bool> {
        self.with_conn(|conn| {
            let rows_affected = conn.execute("DELETE FROM users WHERE id = ?1", [id])?;
            Ok(rows_affected > 0)
        })
    }

    pub fn regenerate_api_key(&self, id: i64) -> Result<Option<(User, String)>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT name FROM users WHERE id = ?1")?;
            let name: Option<String> = stmt.query_row([id], |row| row.get(0)).optional()?;

            let Some(user_name) = name else {
                return Ok(None);
            };

            let (full_key, prefix, hash) = generate_api_key(&user_name);

            conn.execute(
                "UPDATE users SET api_key_prefix = ?1, api_key_hash = ?2 WHERE id = ?3",
                rusqlite::params![prefix, hash, id],
            )?;

            let mut stmt = conn.prepare(
                "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at FROM users WHERE id = ?1",
            )?;
            let user = stmt.query_row([id], |row| row_to_user(row))?;
            Ok(Some((user, full_key)))
        })
    }

    pub fn reset_used_tokens(&self, id: i64) -> Result<Option<i64>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT used_tokens FROM users WHERE id = ?1")?;
            let prev_tokens: Option<i64> = stmt.query_row([id], |row| row.get(0)).optional()?;

            let Some(prev) = prev_tokens else {
                return Ok(None);
            };

            conn.execute("UPDATE users SET used_tokens = 0 WHERE id = ?1", [id])?;
            Ok(Some(prev))
        })
    }

    pub fn list_users_paginated(&self, page: u32, limit: u32) -> Result<(Vec<User>, u64)> {
        self.with_conn(|conn| {
            let total: u64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;

            let offset = (page.saturating_sub(1)) * limit;
            let mut stmt = conn.prepare(
                "SELECT id, name, api_key_prefix, quota_tokens, used_tokens, enabled, created_at, last_used_at 
                 FROM users ORDER BY id LIMIT ?1 OFFSET ?2",
            )?;
            let users = stmt
                .query_map(rusqlite::params![limit, offset], |row| row_to_user(row))?
                .collect::<Result<Vec<_>, _>>()?;

            Ok((users, total))
        })
    }
}
