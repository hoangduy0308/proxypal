use anyhow::Result;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};

use super::Database;
use crate::crypto::{decrypt_tokens, encrypt_tokens};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Provider {
    pub id: i64,
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub settings: serde_json::Value,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderAccount {
    pub id: i64,
    pub provider: String,
    pub account_id: String,
    pub enabled: bool,
    pub created_at: String,
}

fn row_to_provider(row: &rusqlite::Row) -> rusqlite::Result<Provider> {
    let settings_str: String = row.get(4)?;
    let settings: serde_json::Value =
        serde_json::from_str(&settings_str).unwrap_or(serde_json::json!({}));
    Ok(Provider {
        id: row.get(0)?,
        name: row.get(1)?,
        provider_type: row.get(2)?,
        enabled: row.get::<_, i32>(3)? != 0,
        settings,
        created_at: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

fn row_to_provider_account(row: &rusqlite::Row) -> rusqlite::Result<ProviderAccount> {
    Ok(ProviderAccount {
        id: row.get(0)?,
        provider: row.get(1)?,
        account_id: row.get(2)?,
        enabled: row.get::<_, i32>(3)? != 0,
        created_at: row.get(4)?,
    })
}

impl Database {
    pub fn create_provider(
        &self,
        name: &str,
        provider_type: &str,
        enabled: bool,
        settings: &serde_json::Value,
    ) -> Result<Provider> {
        self.with_conn(|conn| {
            let settings_str = serde_json::to_string(settings)?;
            conn.execute(
                "INSERT INTO providers (name, type, enabled, settings, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, datetime('now'), datetime('now'))",
                rusqlite::params![name, provider_type, enabled as i32, settings_str],
            )?;

            let id = conn.last_insert_rowid();
            let mut stmt = conn.prepare(
                "SELECT id, name, type, enabled, settings, created_at, updated_at FROM providers WHERE id = ?1",
            )?;
            let provider = stmt.query_row([id], |row| row_to_provider(row))?;
            Ok(provider)
        })
    }

    pub fn get_provider_by_name(&self, name: &str) -> Result<Option<Provider>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, type, enabled, settings, created_at, updated_at FROM providers WHERE name = ?1",
            )?;
            let provider = stmt
                .query_row([name], |row| row_to_provider(row))
                .optional()?;
            Ok(provider)
        })
    }

    pub fn list_providers(&self) -> Result<Vec<Provider>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, name, type, enabled, settings, created_at, updated_at FROM providers ORDER BY id",
            )?;
            let providers = stmt
                .query_map([], |row| row_to_provider(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(providers)
        })
    }

    pub fn update_provider(
        &self,
        name: &str,
        enabled: Option<bool>,
        settings: Option<&serde_json::Value>,
    ) -> Result<Option<Provider>> {
        self.with_conn(|conn| {
            let mut updates = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(e) = enabled {
                updates.push("enabled = ?");
                params.push(Box::new(e as i32));
            }
            if let Some(s) = settings {
                updates.push("settings = ?");
                params.push(Box::new(serde_json::to_string(s)?));
            }

            if updates.is_empty() {
                return self.get_provider_by_name(name);
            }

            updates.push("updated_at = datetime('now')");
            params.push(Box::new(name.to_string()));

            let sql = format!(
                "UPDATE providers SET {} WHERE name = ?",
                updates.join(", ")
            );

            let params_ref: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let rows_affected = conn.execute(&sql, params_ref.as_slice())?;

            if rows_affected == 0 {
                return Ok(None);
            }

            let mut stmt = conn.prepare(
                "SELECT id, name, type, enabled, settings, created_at, updated_at FROM providers WHERE name = ?1",
            )?;
            let provider = stmt
                .query_row([name], |row| row_to_provider(row))
                .optional()?;
            Ok(provider)
        })
    }

    pub fn delete_provider(&self, name: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let rows_affected = conn.execute("DELETE FROM providers WHERE name = ?1", [name])?;
            Ok(rows_affected > 0)
        })
    }

    pub fn create_provider_account(
        &self,
        provider: &str,
        account_id: &str,
        tokens: &serde_json::Value,
    ) -> Result<ProviderAccount> {
        let encrypted_tokens = encrypt_tokens(tokens)?;

        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO provider_accounts (provider, account_id, tokens, enabled, created_at)
                 VALUES (?1, ?2, ?3, 1, datetime('now'))",
                rusqlite::params![provider, account_id, encrypted_tokens],
            )?;

            let id = conn.last_insert_rowid();
            let mut stmt = conn.prepare(
                "SELECT id, provider, account_id, enabled, created_at FROM provider_accounts WHERE id = ?1",
            )?;
            let account = stmt.query_row([id], |row| row_to_provider_account(row))?;
            Ok(account)
        })
    }

    pub fn get_provider_account(
        &self,
        provider: &str,
        account_id: &str,
    ) -> Result<Option<ProviderAccount>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, account_id, enabled, created_at FROM provider_accounts WHERE provider = ?1 AND account_id = ?2",
            )?;
            let account = stmt
                .query_row(rusqlite::params![provider, account_id], |row| {
                    row_to_provider_account(row)
                })
                .optional()?;
            Ok(account)
        })
    }

    pub fn list_provider_accounts(&self, provider: &str) -> Result<Vec<ProviderAccount>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, provider, account_id, enabled, created_at FROM provider_accounts WHERE provider = ?1 ORDER BY id",
            )?;
            let accounts = stmt
                .query_map([provider], |row| row_to_provider_account(row))?
                .collect::<Result<Vec<_>, _>>()?;
            Ok(accounts)
        })
    }

    pub fn update_provider_account_tokens(
        &self,
        provider: &str,
        account_id: &str,
        tokens: &serde_json::Value,
    ) -> Result<bool> {
        let encrypted_tokens = encrypt_tokens(tokens)?;

        self.with_conn(|conn| {
            let rows_affected = conn.execute(
                "UPDATE provider_accounts SET tokens = ?1 WHERE provider = ?2 AND account_id = ?3",
                rusqlite::params![encrypted_tokens, provider, account_id],
            )?;
            Ok(rows_affected > 0)
        })
    }

    pub fn delete_provider_account(&self, provider: &str, account_id: &str) -> Result<bool> {
        self.with_conn(|conn| {
            let rows_affected = conn.execute(
                "DELETE FROM provider_accounts WHERE provider = ?1 AND account_id = ?2",
                rusqlite::params![provider, account_id],
            )?;
            Ok(rows_affected > 0)
        })
    }

    pub fn get_provider_account_tokens(
        &self,
        provider: &str,
        account_id: &str,
    ) -> Result<Option<serde_json::Value>> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT tokens FROM provider_accounts WHERE provider = ?1 AND account_id = ?2",
            )?;
            let encrypted: Option<String> = stmt
                .query_row(rusqlite::params![provider, account_id], |row| row.get(0))
                .optional()?;

            match encrypted {
                Some(enc) => {
                    let tokens = decrypt_tokens(&enc)?;
                    Ok(Some(tokens))
                }
                None => Ok(None),
            }
        })
    }

    pub fn count_provider_accounts(&self, provider: &str) -> Result<i64> {
        self.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM provider_accounts WHERE provider = ?1",
                [provider],
                |row| row.get(0),
            )?;
            Ok(count)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde_json::json;
    use serial_test::serial;

    fn setup_test_env() {
        let key = [0u8; 32];
        std::env::set_var("ENCRYPTION_KEY", STANDARD.encode(key));
    }

    #[test]
    #[serial]
    fn create_and_get_provider() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        let settings = json!({"client_id": "test123"});
        let provider = db
            .create_provider("openai", "oauth", true, &settings)
            .unwrap();

        assert_eq!(provider.name, "openai");
        assert_eq!(provider.provider_type, "oauth");
        assert!(provider.enabled);
        assert_eq!(provider.settings["client_id"], "test123");

        let fetched = db.get_provider_by_name("openai").unwrap().unwrap();
        assert_eq!(fetched.id, provider.id);
        assert_eq!(fetched.name, "openai");
    }

    #[test]
    #[serial]
    fn list_providers_returns_all() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        db.create_provider("openai", "oauth", true, &json!({}))
            .unwrap();
        db.create_provider("anthropic", "api_key", true, &json!({}))
            .unwrap();
        db.create_provider("google", "oauth", false, &json!({}))
            .unwrap();

        let providers = db.list_providers().unwrap();
        assert_eq!(providers.len(), 3);

        let names: Vec<&str> = providers.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"openai"));
        assert!(names.contains(&"anthropic"));
        assert!(names.contains(&"google"));
    }

    #[test]
    #[serial]
    fn update_provider_settings_and_enabled() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        db.create_provider("openai", "oauth", true, &json!({"key": "old"}))
            .unwrap();

        let new_settings = json!({"key": "new", "extra": true});
        let updated = db
            .update_provider("openai", Some(false), Some(&new_settings))
            .unwrap()
            .unwrap();

        assert!(!updated.enabled);
        assert_eq!(updated.settings["key"], "new");
        assert_eq!(updated.settings["extra"], true);

        let fetched = db.get_provider_by_name("openai").unwrap().unwrap();
        assert!(!fetched.enabled);
        assert_eq!(fetched.settings["key"], "new");
    }

    #[test]
    #[serial]
    fn delete_provider_removes_record() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        db.create_provider("openai", "oauth", true, &json!({}))
            .unwrap();
        assert!(db.get_provider_by_name("openai").unwrap().is_some());

        let deleted = db.delete_provider("openai").unwrap();
        assert!(deleted);

        assert!(db.get_provider_by_name("openai").unwrap().is_none());

        let deleted_again = db.delete_provider("openai").unwrap();
        assert!(!deleted_again);
    }

    #[test]
    #[serial]
    fn create_provider_account_stores_encrypted_tokens() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        let tokens = json!({
            "access_token": "secret_access",
            "refresh_token": "secret_refresh"
        });

        let account = db
            .create_provider_account("openai", "user@example.com", &tokens)
            .unwrap();

        assert_eq!(account.provider, "openai");
        assert_eq!(account.account_id, "user@example.com");
        assert!(account.enabled);

        db.with_conn(|conn| {
            let raw_tokens: String = conn.query_row(
                "SELECT tokens FROM provider_accounts WHERE id = ?1",
                [account.id],
                |row| row.get(0),
            )?;
            assert!(!raw_tokens.contains("secret_access"));
            assert!(!raw_tokens.contains("secret_refresh"));
            Ok(())
        })
        .unwrap();

        let decrypted = db
            .get_provider_account_tokens("openai", "user@example.com")
            .unwrap()
            .unwrap();
        assert_eq!(decrypted["access_token"], "secret_access");
        assert_eq!(decrypted["refresh_token"], "secret_refresh");
    }

    #[test]
    #[serial]
    fn list_accounts_by_provider() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        db.create_provider_account("openai", "user1@example.com", &json!({"token": "1"}))
            .unwrap();
        db.create_provider_account("openai", "user2@example.com", &json!({"token": "2"}))
            .unwrap();
        db.create_provider_account("anthropic", "user3@example.com", &json!({"token": "3"}))
            .unwrap();

        let openai_accounts = db.list_provider_accounts("openai").unwrap();
        assert_eq!(openai_accounts.len(), 2);

        let anthropic_accounts = db.list_provider_accounts("anthropic").unwrap();
        assert_eq!(anthropic_accounts.len(), 1);

        let google_accounts = db.list_provider_accounts("google").unwrap();
        assert_eq!(google_accounts.len(), 0);
    }

    #[test]
    #[serial]
    fn update_provider_account_tokens() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        let old_tokens = json!({"access_token": "old_token"});
        db.create_provider_account("openai", "user@example.com", &old_tokens)
            .unwrap();

        let new_tokens = json!({"access_token": "new_token", "refresh_token": "refresh"});
        let updated = db
            .update_provider_account_tokens("openai", "user@example.com", &new_tokens)
            .unwrap();
        assert!(updated);

        let fetched = db
            .get_provider_account_tokens("openai", "user@example.com")
            .unwrap()
            .unwrap();
        assert_eq!(fetched["access_token"], "new_token");
        assert_eq!(fetched["refresh_token"], "refresh");

        let not_found = db
            .update_provider_account_tokens("openai", "nonexistent@example.com", &new_tokens)
            .unwrap();
        assert!(!not_found);
    }

    #[test]
    #[serial]
    fn delete_provider_account() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        db.create_provider_account("openai", "user@example.com", &json!({"token": "test"}))
            .unwrap();

        assert!(db
            .get_provider_account("openai", "user@example.com")
            .unwrap()
            .is_some());
        assert_eq!(db.count_provider_accounts("openai").unwrap(), 1);

        let deleted = db
            .delete_provider_account("openai", "user@example.com")
            .unwrap();
        assert!(deleted);

        assert!(db
            .get_provider_account("openai", "user@example.com")
            .unwrap()
            .is_none());
        assert_eq!(db.count_provider_accounts("openai").unwrap(), 0);

        let deleted_again = db
            .delete_provider_account("openai", "user@example.com")
            .unwrap();
        assert!(!deleted_again);
    }
}
