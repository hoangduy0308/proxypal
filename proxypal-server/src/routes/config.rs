use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::cliproxy::{
    generate_proxy_config, load_server_config, save_server_config, ServerConfig,
};
use crate::middleware::admin_auth::AdminSession;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateConfigResponse {
    pub success: bool,
    pub restart_required: bool,
}

#[derive(Debug)]
pub enum ConfigError {
    ValidationError(String),
    Internal(String),
}

impl IntoResponse for ConfigError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match self {
            Self::ValidationError(msg) => (StatusCode::BAD_REQUEST, "VALIDATION_ERROR", msg),
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        let body = serde_json::json!({
            "success": false,
            "error": message,
            "code": code
        });

        (status, Json(body)).into_response()
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct UpdateConfigRequest {
    pub proxy_port: Option<u16>,
    pub admin_port: Option<u16>,
    pub log_level: Option<String>,
    pub auto_start_proxy: Option<bool>,
    pub model_mappings: Option<HashMap<String, String>>,
    pub rate_limits: Option<RateLimitsRequest>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RateLimitsRequest {
    pub requests_per_minute: Option<u64>,
    pub tokens_per_day: Option<i64>,
}

pub async fn get_config(
    _session: AdminSession,
    State(state): State<AppState>,
) -> Result<Json<ServerConfig>, ConfigError> {
    load_server_config(&state.db)
        .map(Json)
        .map_err(|e| ConfigError::Internal(e.to_string()))
}

pub async fn update_config(
    _session: AdminSession,
    State(state): State<AppState>,
    Json(payload): Json<UpdateConfigRequest>,
) -> Result<Json<UpdateConfigResponse>, ConfigError> {
    let mut config =
        load_server_config(&state.db).map_err(|e| ConfigError::Internal(e.to_string()))?;

    let old_admin_port = config.admin_port;
    let old_proxy_port = config.proxy_port;

    if let Some(port) = payload.proxy_port {
        validate_port(port)?;
        config.proxy_port = port;
    }
    if let Some(port) = payload.admin_port {
        validate_port(port)?;
        config.admin_port = port;
    }
    if let Some(level) = payload.log_level {
        validate_log_level(&level)?;
        config.log_level = level;
    }
    if let Some(auto_start) = payload.auto_start_proxy {
        config.auto_start_proxy = auto_start;
    }
    if let Some(mappings) = payload.model_mappings {
        config.model_mappings = mappings;
    }
    if let Some(limits) = payload.rate_limits {
        if let Some(rpm) = limits.requests_per_minute {
            config.rate_limits.requests_per_minute = rpm;
        }
        if let Some(tpd) = limits.tokens_per_day {
            config.rate_limits.tokens_per_day = Some(tpd);
        }
    }

    save_server_config(&state.db, &config).map_err(|e| ConfigError::Internal(e.to_string()))?;

    let restart_required = config.admin_port != old_admin_port;

    if config.proxy_port == old_proxy_port {
        let config_path = get_proxy_config_path();
        if let Err(e) = generate_proxy_config(&state.db, &config, &config_path) {
            tracing::error!("Failed to regenerate proxy config: {}", e);
        }
        if let Err(e) = state.proxy_client.sync_provider("*").await {
            tracing::warn!("Failed to hot-reload proxy: {}", e);
        }
    }

    Ok(Json(UpdateConfigResponse {
        success: true,
        restart_required,
    }))
}

fn validate_port(port: u16) -> Result<(), ConfigError> {
    if port < 1024 && port != 0 {
        return Err(ConfigError::ValidationError(
            "Port must be >= 1024 (or 0 for auto)".to_string(),
        ));
    }
    Ok(())
}

fn validate_log_level(level: &str) -> Result<(), ConfigError> {
    match level {
        "trace" | "debug" | "info" | "warn" | "error" => Ok(()),
        _ => Err(ConfigError::ValidationError(format!(
            "Invalid log level: {}",
            level
        ))),
    }
}

fn get_proxy_config_path() -> std::path::PathBuf {
    std::env::var("DATA_DIR")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("/data"))
        .join("proxy-config.yaml")
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_config).put(update_config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
    use crate::db::Database;
    use crate::middleware::rate_limit::RateLimiter;
    use axum::{body::Body, http::Request};
    use std::sync::Arc;
    use tempfile::{tempdir, TempDir};
    use tower::ServiceExt;

    fn create_test_db() -> (Database, TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path).unwrap();
        (db, dir)
    }

    fn create_app(db: Database) -> (Router, String) {
        let session_id = "test-session-id";
        let csrf_token = "test-csrf-token";
        db.create_session(session_id, csrf_token, 7).unwrap();

        let state = AppState {
            db,
            rate_limiter: Arc::new(RateLimiter::new(60)),
            proxy_client: Arc::new(MockProxyManagementClient::default()),
            proxy_manager: Arc::new(MockProxyProcessManager::default()),
        };
        let app = Router::new()
            .nest("/api/config", router())
            .with_state(state);

        (app, session_id.to_string())
    }

    fn authed_request(
        method: &str,
        uri: &str,
        session_id: &str,
        body: Option<&str>,
    ) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("Cookie", format!("session={}", session_id));

        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }

        builder
            .body(
                body.map(|b| Body::from(b.to_string()))
                    .unwrap_or(Body::empty()),
            )
            .unwrap()
    }

    #[tokio::test]
    async fn test_get_config_returns_defaults_when_not_set() {
        let (db, _dir) = create_test_db();
        let config = load_server_config(&db).unwrap();

        assert_eq!(config.proxy_port, 8317);
        assert_eq!(config.admin_port, 3000);
        assert_eq!(config.log_level, "info");
    }

    #[tokio::test]
    async fn test_update_config_persists_changes() {
        let (db, _dir) = create_test_db();

        let mut config = load_server_config(&db).unwrap();
        config.proxy_port = 9999;
        config.log_level = "debug".to_string();
        save_server_config(&db, &config).unwrap();

        let loaded = load_server_config(&db).unwrap();
        assert_eq!(loaded.proxy_port, 9999);
        assert_eq!(loaded.log_level, "debug");
    }

    #[test]
    fn test_validate_port_rejects_privileged_ports() {
        assert!(validate_port(80).is_err());
        assert!(validate_port(443).is_err());
        assert!(validate_port(1023).is_err());
        assert!(validate_port(1024).is_ok());
        assert!(validate_port(8317).is_ok());
        assert!(validate_port(0).is_ok());
    }

    #[test]
    fn test_validate_log_level() {
        assert!(validate_log_level("info").is_ok());
        assert!(validate_log_level("debug").is_ok());
        assert!(validate_log_level("trace").is_ok());
        assert!(validate_log_level("warn").is_ok());
        assert!(validate_log_level("error").is_ok());
        assert!(validate_log_level("invalid").is_err());
    }

    #[tokio::test]
    async fn test_get_config_via_http() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/config", &session_id, None);
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ServerConfig = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.proxy_port, 8317);
        assert_eq!(json.admin_port, 3000);
        assert_eq!(json.log_level, "info");
    }

    #[tokio::test]
    async fn test_update_config_via_http() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request(
            "PUT",
            "/api/config",
            &session_id,
            Some(r#"{"proxy_port": 9000, "log_level": "debug"}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: UpdateConfigResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.success);
    }

    #[tokio::test]
    async fn test_update_config_returns_restart_required_when_admin_port_changes() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request(
            "PUT",
            "/api/config",
            &session_id,
            Some(r#"{"admin_port": 4000}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: UpdateConfigResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.success);
        assert!(json.restart_required);
    }

    #[tokio::test]
    async fn test_update_config_rejects_invalid_port() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request(
            "PUT",
            "/api/config",
            &session_id,
            Some(r#"{"proxy_port": 80}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_update_config_rejects_invalid_log_level() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request(
            "PUT",
            "/api/config",
            &session_id,
            Some(r#"{"log_level": "invalid"}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_unauthenticated_request_returns_401() {
        let (db, _dir) = create_test_db();
        let state = AppState {
            db,
            rate_limiter: Arc::new(RateLimiter::new(60)),
            proxy_client: Arc::new(MockProxyManagementClient::default()),
            proxy_manager: Arc::new(MockProxyProcessManager::default()),
        };
        let app = Router::new()
            .nest("/api/config", router())
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/config")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_update_config_with_rate_limits() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db.clone());

        let request = authed_request(
            "PUT",
            "/api/config",
            &session_id,
            Some(r#"{"rate_limits": {"requests_per_minute": 120, "tokens_per_day": 1000000}}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let loaded = load_server_config(&db).unwrap();
        assert_eq!(loaded.rate_limits.requests_per_minute, 120);
        assert_eq!(loaded.rate_limits.tokens_per_day, Some(1000000));
    }

    #[tokio::test]
    async fn test_update_config_with_model_mappings() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db.clone());

        let request = authed_request(
            "PUT",
            "/api/config",
            &session_id,
            Some(r#"{"model_mappings": {"gpt-4": "claude-3-opus", "gpt-3.5": "claude-3-sonnet"}}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let loaded = load_server_config(&db).unwrap();
        assert_eq!(loaded.model_mappings.len(), 2);
        assert_eq!(
            loaded.model_mappings.get("gpt-4"),
            Some(&"claude-3-opus".to_string())
        );
    }
}
