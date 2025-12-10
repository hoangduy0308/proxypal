use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use crate::db::providers::{Provider, ProviderAccount};
use crate::db::Database;

const SERVER_CONFIG_KEY: &str = "server_config";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ServerConfig {
    pub proxy_port: u16,
    pub admin_port: u16,
    pub log_level: String,
    pub auto_start_proxy: bool,
    pub model_mappings: HashMap<String, String>,
    pub rate_limits: RateLimits,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            proxy_port: 8317,
            admin_port: 3000,
            log_level: "info".to_string(),
            auto_start_proxy: true,
            model_mappings: HashMap::new(),
            rate_limits: RateLimits::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimits {
    pub requests_per_minute: u64,
    pub tokens_per_day: Option<i64>,
}

impl Default for RateLimits {
    fn default() -> Self {
        Self {
            requests_per_minute: 60,
            tokens_per_day: None,
        }
    }
}

pub fn load_server_config(db: &Database) -> Result<ServerConfig> {
    match db.get_setting(SERVER_CONFIG_KEY)? {
        Some(json_str) => {
            let config: ServerConfig = serde_json::from_str(&json_str)?;
            Ok(config)
        }
        None => Ok(ServerConfig::default()),
    }
}

pub fn save_server_config(db: &Database, config: &ServerConfig) -> Result<()> {
    let json_str = serde_json::to_string(config)?;
    db.set_setting(SERVER_CONFIG_KEY, &json_str)?;
    Ok(())
}

pub fn build_proxy_config_yaml(
    server_config: &ServerConfig,
    providers: Vec<Provider>,
    accounts: Vec<ProviderAccount>,
) -> Result<String> {
    let mut yaml_parts = Vec::new();

    yaml_parts.push(format!("port: {}", server_config.proxy_port));
    yaml_parts.push(format!("log-level: {}", server_config.log_level));
    yaml_parts.push("auth-dir: ./auth".to_string());
    yaml_parts.push("api-keys:".to_string());
    yaml_parts.push("  - proxypal-default-key".to_string());

    if !server_config.model_mappings.is_empty() {
        yaml_parts.push("model-mappings:".to_string());
        for (from, to) in &server_config.model_mappings {
            yaml_parts.push(format!("  {}: {}", from, to));
        }
    }

    let enabled_providers: Vec<_> = providers.iter().filter(|p| p.enabled).collect();
    if !enabled_providers.is_empty() {
        yaml_parts.push("providers:".to_string());
        for provider in enabled_providers {
            let provider_accounts: Vec<_> = accounts
                .iter()
                .filter(|a| a.provider == provider.name && a.enabled)
                .collect();

            if !provider_accounts.is_empty() {
                yaml_parts.push(format!("  {}:", provider.name));
                yaml_parts.push("    enabled: true".to_string());
                yaml_parts.push(format!("    accounts: {}", provider_accounts.len()));
            }
        }
    }

    if server_config.rate_limits.requests_per_minute > 0 {
        yaml_parts.push("rate-limits:".to_string());
        yaml_parts.push(format!(
            "  requests-per-minute: {}",
            server_config.rate_limits.requests_per_minute
        ));
        if let Some(tokens) = server_config.rate_limits.tokens_per_day {
            yaml_parts.push(format!("  tokens-per-day: {}", tokens));
        }
    }

    Ok(yaml_parts.join("\n"))
}

pub fn generate_proxy_config(
    db: &Database,
    server_config: &ServerConfig,
    config_path: &Path,
) -> Result<()> {
    let providers = db.list_providers()?;

    let mut accounts = Vec::new();
    for provider in &providers {
        let provider_accounts = db.list_provider_accounts(&provider.name)?;
        accounts.extend(provider_accounts);
    }

    let yaml_content = build_proxy_config_yaml(server_config, providers, accounts)?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(config_path, yaml_content)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serial_test::serial;
    use tempfile::tempdir;

    fn setup_test_env() {
        let key = [0u8; 32];
        std::env::set_var("ENCRYPTION_KEY", STANDARD.encode(key));
    }

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.proxy_port, 8317);
        assert_eq!(config.admin_port, 3000);
        assert_eq!(config.log_level, "info");
        assert!(config.auto_start_proxy);
        assert_eq!(config.rate_limits.requests_per_minute, 60);
        assert!(config.rate_limits.tokens_per_day.is_none());
    }

    #[test]
    fn test_load_server_config_returns_defaults_when_not_set() {
        let db = Database::new_in_memory().unwrap();
        let config = load_server_config(&db).unwrap();
        assert_eq!(config.proxy_port, 8317);
        assert_eq!(config.admin_port, 3000);
        assert_eq!(config.log_level, "info");
    }

    #[test]
    fn test_save_and_load_server_config() {
        let db = Database::new_in_memory().unwrap();
        let mut config = ServerConfig::default();
        config.proxy_port = 8888;
        config.log_level = "debug".to_string();

        save_server_config(&db, &config).unwrap();
        let loaded = load_server_config(&db).unwrap();

        assert_eq!(loaded.proxy_port, 8888);
        assert_eq!(loaded.log_level, "debug");
    }

    #[test]
    fn test_save_and_load_with_model_mappings() {
        let db = Database::new_in_memory().unwrap();
        let mut config = ServerConfig::default();
        config
            .model_mappings
            .insert("gpt-4".to_string(), "claude-3-opus".to_string());
        config.model_mappings.insert(
            "gpt-3.5-turbo".to_string(),
            "claude-3-sonnet".to_string(),
        );

        save_server_config(&db, &config).unwrap();
        let loaded = load_server_config(&db).unwrap();

        assert_eq!(loaded.model_mappings.len(), 2);
        assert_eq!(
            loaded.model_mappings.get("gpt-4"),
            Some(&"claude-3-opus".to_string())
        );
    }

    #[test]
    fn test_build_proxy_config_yaml_basic() {
        let config = ServerConfig::default();
        let yaml = build_proxy_config_yaml(&config, vec![], vec![]).unwrap();

        assert!(yaml.contains("port: 8317"));
        assert!(yaml.contains("log-level: info"));
        assert!(yaml.contains("auth-dir:"));
        assert!(yaml.contains("api-keys:"));
    }

    #[test]
    fn test_build_proxy_config_yaml_with_model_mappings() {
        let mut config = ServerConfig::default();
        config
            .model_mappings
            .insert("gpt-4".to_string(), "claude-3-opus".to_string());

        let yaml = build_proxy_config_yaml(&config, vec![], vec![]).unwrap();
        assert!(yaml.contains("model-mappings:"));
        assert!(yaml.contains("gpt-4"));
        assert!(yaml.contains("claude-3-opus"));
    }

    #[test]
    fn test_build_proxy_config_yaml_with_rate_limits() {
        let mut config = ServerConfig::default();
        config.rate_limits.requests_per_minute = 120;
        config.rate_limits.tokens_per_day = Some(1000000);

        let yaml = build_proxy_config_yaml(&config, vec![], vec![]).unwrap();
        assert!(yaml.contains("rate-limits:"));
        assert!(yaml.contains("requests-per-minute: 120"));
        assert!(yaml.contains("tokens-per-day: 1000000"));
    }

    #[test]
    #[serial]
    fn test_generate_proxy_config_writes_file() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();
        let config = ServerConfig::default();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("proxy-config.yaml");

        generate_proxy_config(&db, &config, &config_path).unwrap();

        assert!(config_path.exists());
        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("port: 8317"));
        assert!(content.contains("log-level: info"));
    }

    #[test]
    #[serial]
    fn test_generate_proxy_config_with_providers() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();

        db.create_provider("google", "oauth", true, &serde_json::json!({}))
            .unwrap();
        db.create_provider_account("google", "user@gmail.com", &serde_json::json!({"token": "test"}))
            .unwrap();

        let config = ServerConfig::default();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("proxy-config.yaml");

        generate_proxy_config(&db, &config, &config_path).unwrap();

        let content = std::fs::read_to_string(&config_path).unwrap();
        assert!(content.contains("providers:"));
        assert!(content.contains("google:"));
    }

    #[test]
    #[serial]
    fn test_generate_proxy_config_creates_parent_dirs() {
        setup_test_env();
        let db = Database::new_in_memory().unwrap();
        let config = ServerConfig::default();
        let dir = tempdir().unwrap();
        let config_path = dir.path().join("nested").join("dir").join("proxy-config.yaml");

        generate_proxy_config(&db, &config, &config_path).unwrap();

        assert!(config_path.exists());
    }
}
