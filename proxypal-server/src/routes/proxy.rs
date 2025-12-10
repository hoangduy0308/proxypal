use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::cliproxy::config_gen::{generate_proxy_config, load_server_config};
use crate::middleware::admin_auth::AdminSession;
use crate::AppState;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ProxyStatusResponse {
    pub running: bool,
    pub pid: Option<u32>,
    pub port: u16,
    pub uptime_seconds: Option<u64>,
    pub total_requests: i64,
    pub active_providers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartProxyResponse {
    pub success: bool,
    pub pid: Option<u32>,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopProxyResponse {
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestartProxyResponse {
    pub success: bool,
    pub pid: Option<u32>,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub code: String,
}

#[derive(Debug)]
pub enum ProxyError {
    Internal(String),
    Conflict(String),
}

impl IntoResponse for ProxyError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match self {
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
            Self::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg),
        };

        let body = Json(ErrorResponse {
            success: false,
            error: message,
            code: code.to_string(),
        });

        (status, body).into_response()
    }
}

fn get_config_path() -> PathBuf {
    std::env::var("PROXY_CONFIG_PATH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./proxy-config.yaml"))
}

pub async fn get_proxy_status(
    _session: AdminSession,
    State(state): State<AppState>,
) -> Result<Json<ProxyStatusResponse>, ProxyError> {
    let server_config = load_server_config(&state.db)
        .map_err(|e| ProxyError::Internal(format!("Failed to load server config: {}", e)))?;

    let running = state.proxy_manager.is_running();
    let pid = state.proxy_manager.pid();
    let uptime_seconds = state.proxy_manager.uptime_seconds();

    let active_providers = if running {
        state
            .db
            .list_providers()
            .map_err(|e| ProxyError::Internal(format!("Failed to list providers: {}", e)))?
            .into_iter()
            .filter(|p| p.enabled)
            .map(|p| p.name)
            .collect()
    } else {
        Vec::new()
    };

    let total_requests = state.db.get_total_requests().unwrap_or(0);

    Ok(Json(ProxyStatusResponse {
        running,
        pid,
        port: server_config.proxy_port,
        uptime_seconds,
        total_requests,
        active_providers,
    }))
}

pub async fn start_proxy(
    _session: AdminSession,
    State(state): State<AppState>,
) -> Result<(StatusCode, Json<StartProxyResponse>), ProxyError> {
    if state.proxy_manager.is_running() {
        return Err(ProxyError::Conflict("Proxy is already running".to_string()));
    }

    let server_config = load_server_config(&state.db)
        .map_err(|e| ProxyError::Internal(format!("Failed to load server config: {}", e)))?;

    let config_path = get_config_path();

    generate_proxy_config(&state.db, &server_config, &config_path)
        .map_err(|e| ProxyError::Internal(format!("Failed to generate proxy config: {}", e)))?;

    let pid = state
        .proxy_manager
        .start(&config_path, server_config.proxy_port)
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to start proxy: {}", e)))?;

    Ok((
        StatusCode::OK,
        Json(StartProxyResponse {
            success: true,
            pid: Some(pid),
            port: server_config.proxy_port,
            error: None,
        }),
    ))
}

pub async fn stop_proxy(
    _session: AdminSession,
    State(state): State<AppState>,
) -> Result<Json<StopProxyResponse>, ProxyError> {
    let _ = state.proxy_manager.stop().await;

    Ok(Json(StopProxyResponse { success: true }))
}

pub async fn restart_proxy(
    _session: AdminSession,
    State(state): State<AppState>,
) -> Result<Json<RestartProxyResponse>, ProxyError> {
    let _ = state.proxy_manager.stop().await;

    let server_config = load_server_config(&state.db)
        .map_err(|e| ProxyError::Internal(format!("Failed to load server config: {}", e)))?;

    let config_path = get_config_path();

    generate_proxy_config(&state.db, &server_config, &config_path)
        .map_err(|e| ProxyError::Internal(format!("Failed to generate proxy config: {}", e)))?;

    let pid = state
        .proxy_manager
        .start(&config_path, server_config.proxy_port)
        .await
        .map_err(|e| ProxyError::Internal(format!("Failed to start proxy: {}", e)))?;

    Ok(Json(RestartProxyResponse {
        success: true,
        pid: Some(pid),
        port: server_config.proxy_port,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/status", get(get_proxy_status))
        .route("/start", post(start_proxy))
        .route("/stop", post(stop_proxy))
        .route("/restart", post(restart_proxy))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
    use crate::db::Database;
    use crate::middleware::rate_limit::RateLimiter;
    use axum::{body::Body, http::Request};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use std::sync::Arc;
    use tower::ServiceExt;

    fn setup_test_env() {
        let key = [0u8; 32];
        std::env::set_var("ENCRYPTION_KEY", STANDARD.encode(key));
    }

    fn create_test_db() -> Database {
        setup_test_env();
        Database::new_in_memory().unwrap()
    }

    fn create_test_state() -> AppState {
        let db = create_test_db();
        let rate_limiter = Arc::new(RateLimiter::new(60));
        let proxy_client: Arc<dyn crate::cliproxy::ProxyManagementClient> =
            Arc::new(MockProxyManagementClient::default());
        let proxy_manager: Arc<dyn crate::cliproxy::ProxyProcessManager> =
            Arc::new(MockProxyProcessManager::default());

        AppState {
            db,
            rate_limiter,
            proxy_client,
            proxy_manager,
        }
    }

    fn create_app(state: AppState) -> axum::Router {
        axum::Router::new()
            .nest("/api/proxy", router())
            .with_state(state)
    }

    fn create_app_with_session(state: AppState, session_id: &str) -> (axum::Router, String) {
        state
            .db
            .create_session(session_id, "csrf-token", 7)
            .unwrap();
        (create_app(state), session_id.to_string())
    }

    #[tokio::test]
    async fn test_get_proxy_status_requires_admin_session() {
        let state = create_test_state();
        let app = create_app(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/proxy/status")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_proxy_status_when_not_running() {
        let state = create_test_state();
        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/proxy/status")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ProxyStatusResponse = serde_json::from_slice(&body).unwrap();

        assert!(!json.running);
        assert_eq!(json.pid, None);
        assert_eq!(json.port, 8317);
        assert_eq!(json.uptime_seconds, None);
    }

    #[tokio::test]
    async fn test_get_proxy_status_when_running() {
        let state = create_test_state();

        {
            let mock_manager = state
                .proxy_manager
                .as_any()
                .downcast_ref::<MockProxyProcessManager>()
                .unwrap();
            mock_manager.set_running(true, 12345);
        }

        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/proxy/status")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ProxyStatusResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.running);
        assert_eq!(json.pid, Some(12345));
        assert_eq!(json.uptime_seconds, Some(120));
    }

    #[tokio::test]
    async fn test_start_proxy_requires_admin_session() {
        let state = create_test_state();
        let app = create_app(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/start")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_start_proxy_when_already_running_returns_409() {
        let state = create_test_state();

        {
            let mock_manager = state
                .proxy_manager
                .as_any()
                .downcast_ref::<MockProxyProcessManager>()
                .unwrap();
            mock_manager.set_running(true, 12345);
        }

        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/start")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ErrorResponse = serde_json::from_slice(&body).unwrap();

        assert!(!json.success);
        assert_eq!(json.code, "CONFLICT");
    }

    #[tokio::test]
    async fn test_start_proxy_success() {
        let state = create_test_state();
        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/start")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: StartProxyResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.success);
        assert!(json.pid.is_some());
        assert_eq!(json.port, 8317);
        assert!(json.error.is_none());
    }

    #[tokio::test]
    async fn test_stop_proxy_requires_admin_session() {
        let state = create_test_state();
        let app = create_app(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/stop")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_stop_proxy_succeeds_even_when_not_running() {
        let state = create_test_state();
        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/stop")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: StopProxyResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.success);
    }

    #[tokio::test]
    async fn test_stop_proxy_when_running() {
        let state = create_test_state();

        {
            let mock_manager = state
                .proxy_manager
                .as_any()
                .downcast_ref::<MockProxyProcessManager>()
                .unwrap();
            mock_manager.set_running(true, 12345);
        }

        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/stop")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: StopProxyResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.success);
    }

    #[tokio::test]
    async fn test_restart_proxy_requires_admin_session() {
        let state = create_test_state();
        let app = create_app(state);

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/restart")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_restart_proxy_success() {
        let state = create_test_state();

        {
            let mock_manager = state
                .proxy_manager
                .as_any()
                .downcast_ref::<MockProxyProcessManager>()
                .unwrap();
            mock_manager.set_running(true, 12345);
        }

        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/restart")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: RestartProxyResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.success);
        assert!(json.pid.is_some());
        assert_eq!(json.port, 8317);
    }

    #[tokio::test]
    async fn test_restart_calls_stop_and_start() {
        let state = create_test_state();

        {
            let mock_manager = state
                .proxy_manager
                .as_any()
                .downcast_ref::<MockProxyProcessManager>()
                .unwrap();
            mock_manager.set_running(true, 12345);
        }

        state.db.create_session("test-session", "csrf-token", 7).unwrap();
        let app = create_app(state.clone());

        let request = Request::builder()
            .method("POST")
            .uri("/api/proxy/restart")
            .header("Cookie", "session=test-session")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let mock_manager = state
            .proxy_manager
            .as_any()
            .downcast_ref::<MockProxyProcessManager>()
            .unwrap();
        let calls = mock_manager.call_log.lock().unwrap();
        assert!(calls.iter().any(|c| c == "stop"), "stop should be called");
        assert!(calls.iter().any(|c| c.starts_with("start:")), "start should be called");
    }

    #[tokio::test]
    async fn test_status_includes_active_providers_when_running() {
        let state = create_test_state();

        state
            .db
            .create_provider("claude", "oauth", true, &serde_json::json!({}))
            .unwrap();
        state
            .db
            .create_provider("chatgpt", "oauth", false, &serde_json::json!({}))
            .unwrap();

        {
            let mock_manager = state
                .proxy_manager
                .as_any()
                .downcast_ref::<MockProxyProcessManager>()
                .unwrap();
            mock_manager.set_running(true, 12345);
        }

        let (app, session_id) = create_app_with_session(state, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/proxy/status")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ProxyStatusResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.active_providers.contains(&"claude".to_string()));
        assert!(!json.active_providers.contains(&"chatgpt".to_string()));
    }
}
