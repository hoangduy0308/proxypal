use std::path::PathBuf;
use crate::core::types::{AppConfig, AuthStatus, RequestHistory, AmpOpenAIProvider, generate_uuid};

/// Get config file path, using the provided base directory
pub fn get_config_path_with_base(base_dir: &PathBuf) -> PathBuf {
    let config_dir = base_dir.join("proxypal");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("config.json")
}

/// Get auth file path, using the provided base directory
pub fn get_auth_path_with_base(base_dir: &PathBuf) -> PathBuf {
    let config_dir = base_dir.join("proxypal");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("auth.json")
}

/// Get history file path, using the provided base directory
pub fn get_history_path_with_base(base_dir: &PathBuf) -> PathBuf {
    let config_dir = base_dir.join("proxypal");
    std::fs::create_dir_all(&config_dir).ok();
    config_dir.join("history.json")
}

/// Get the default config directory
pub fn get_default_config_dir() -> PathBuf {
    dirs::config_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Get config file path using default config directory
pub fn get_config_path() -> PathBuf {
    get_config_path_with_base(&get_default_config_dir())
}

/// Get auth file path using default config directory
pub fn get_auth_path() -> PathBuf {
    get_auth_path_with_base(&get_default_config_dir())
}

/// Get history file path using default config directory
pub fn get_history_path() -> PathBuf {
    get_history_path_with_base(&get_default_config_dir())
}

/// Load config from file
pub fn load_config() -> AppConfig {
    load_config_from_path(&get_config_path())
}

/// Load config from a specific path
pub fn load_config_from_path(path: &PathBuf) -> AppConfig {
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(mut config) = serde_json::from_str::<AppConfig>(&data) {
                // Migration: Convert deprecated amp_openai_provider to amp_openai_providers array
                if let Some(old_provider) = config.amp_openai_provider.take() {
                    // Only migrate if the new array is empty (first-time migration)
                    if config.amp_openai_providers.is_empty() {
                        // Ensure the migrated provider has an ID
                        let provider_with_id = if old_provider.id.is_empty() {
                            AmpOpenAIProvider {
                                id: generate_uuid(),
                                ..old_provider
                            }
                        } else {
                            old_provider
                        };
                        config.amp_openai_providers.push(provider_with_id);
                        // Save the migrated config
                        let _ = save_config_to_path(&config, path);
                    }
                }
                return config;
            }
        }
    }
    AppConfig::default()
}

/// Save config to file using default path
pub fn save_config_to_file(config: &AppConfig) -> Result<(), String> {
    save_config_to_path(config, &get_config_path())
}

/// Save config to a specific path
pub fn save_config_to_path(config: &AppConfig, path: &PathBuf) -> Result<(), String> {
    let data = serde_json::to_string_pretty(config).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

/// Load auth status from file
pub fn load_auth_status() -> AuthStatus {
    load_auth_status_from_path(&get_auth_path())
}

/// Load auth status from a specific path
pub fn load_auth_status_from_path(path: &PathBuf) -> AuthStatus {
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(auth) = serde_json::from_str(&data) {
                return auth;
            }
        }
    }
    AuthStatus::default()
}

/// Save auth status to file using default path
pub fn save_auth_to_file(auth: &AuthStatus) -> Result<(), String> {
    save_auth_to_path(auth, &get_auth_path())
}

/// Save auth status to a specific path
pub fn save_auth_to_path(auth: &AuthStatus, path: &PathBuf) -> Result<(), String> {
    let data = serde_json::to_string_pretty(auth).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

/// Load request history from file
pub fn load_request_history() -> RequestHistory {
    load_request_history_from_path(&get_history_path())
}

/// Load request history from a specific path
pub fn load_request_history_from_path(path: &PathBuf) -> RequestHistory {
    if path.exists() {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(history) = serde_json::from_str(&data) {
                return history;
            }
        }
    }
    RequestHistory::default()
}

/// Save request history to file (keep last 500 requests)
pub fn save_request_history(history: &RequestHistory) -> Result<(), String> {
    save_request_history_to_path(history, &get_history_path())
}

/// Save request history to a specific path (keep last 500 requests)
pub fn save_request_history_to_path(history: &RequestHistory, path: &PathBuf) -> Result<(), String> {
    let mut trimmed = history.clone();
    // Keep only last 500 requests
    if trimmed.requests.len() > 500 {
        trimmed.requests = trimmed.requests.split_off(trimmed.requests.len() - 500);
    }
    let data = serde_json::to_string_pretty(&trimmed).map_err(|e| e.to_string())?;
    std::fs::write(path, data).map_err(|e| e.to_string())
}

/// Estimate cost based on model and tokens
pub fn estimate_request_cost(model: &str, tokens_in: u32, tokens_out: u32) -> f64 {
    // Pricing per 1M tokens (input, output) - approximate as of 2024
    // Using broader patterns to match all model versions (3.x, 4.x, 4.5, 5.x, etc.)
    let (input_rate, output_rate) = match model.to_lowercase().as_str() {
        // Claude models - broader patterns to match all versions (3.x, 4.x, 4.5, etc.)
        m if m.contains("claude") && m.contains("opus") => (15.0, 75.0),
        m if m.contains("claude") && m.contains("sonnet") => (3.0, 15.0),
        m if m.contains("claude") && m.contains("haiku") => (0.25, 1.25),
        // GPT models - check newer versions first
        m if m.contains("gpt-5") => (15.0, 45.0),
        m if m.contains("gpt-4o") => (2.5, 10.0),
        m if m.contains("gpt-4-turbo") || m.contains("gpt-4") => (10.0, 30.0),
        m if m.contains("gpt-3.5") => (0.5, 1.5),
        // Gemini models - broader patterns for all 2.x versions
        m if m.contains("gemini") && m.contains("pro") => (1.25, 5.0),
        m if m.contains("gemini") && m.contains("flash") => (0.075, 0.30),
        m if m.contains("gemini-2") => (0.10, 0.40),
        m if m.contains("qwen") => (0.50, 2.0),
        _ => (1.0, 3.0), // Default conservative estimate
    };
    
    let input_cost = (tokens_in as f64 / 1_000_000.0) * input_rate;
    let output_cost = (tokens_out as f64 / 1_000_000.0) * output_rate;
    input_cost + output_cost
}
