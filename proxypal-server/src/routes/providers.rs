use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::middleware::admin_auth::AdminSession;
use crate::AppState;

const VALID_PROVIDERS: &[&str] = &["claude", "chatgpt", "gemini", "copilot"];

fn is_valid_provider(name: &str) -> bool {
    VALID_PROVIDERS.contains(&name.to_lowercase().as_str())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSummary {
    pub name: String,
    pub provider_type: String,
    pub enabled: bool,
    pub accounts_count: i64,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListProvidersResponse {
    pub providers: Vec<ProviderSummary>,
    pub total: usize,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuthStartResponse {
    pub auth_url: String,
    pub state: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderStatusResponse {
    pub name: String,
    pub status: String,
    pub accounts_count: i64,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProviderSettingsRequest {
    pub settings: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SuccessResponse {
    pub success: bool,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub code: String,
}

#[derive(Debug)]
pub enum ProviderError {
    NotFound(String),
    InvalidProvider(String),
    DatabaseError(String),
    ProxyError(String),
}

impl IntoResponse for ProviderError {
    fn into_response(self) -> Response {
        let (status, error, code) = match self {
            ProviderError::NotFound(name) => (
                StatusCode::NOT_FOUND,
                format!("Provider '{}' not found", name),
                "NOT_FOUND".to_string(),
            ),
            ProviderError::InvalidProvider(name) => (
                StatusCode::BAD_REQUEST,
                format!("Invalid provider: '{}'. Supported: claude, chatgpt, gemini, copilot", name),
                "INVALID_PROVIDER".to_string(),
            ),
            ProviderError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
                "INTERNAL_ERROR".to_string(),
            ),
            ProviderError::ProxyError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Proxy error: {}", e),
                "PROXY_ERROR".to_string(),
            ),
        };

        let body = Json(ErrorResponse {
            success: false,
            error,
            code,
        });

        (status, body).into_response()
    }
}

// OAuth Flow (Phase 3.1)

pub async fn start_oauth(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(provider): Path<String>,
) -> Result<Json<OAuthStartResponse>, ProviderError> {
    if !is_valid_provider(&provider) {
        return Err(ProviderError::InvalidProvider(provider));
    }

    let (auth_url, oauth_state) = state
        .proxy_client
        .start_oauth(&provider, true)
        .await
        .map_err(|e| ProviderError::ProxyError(e.to_string()))?;

    Ok(Json(OAuthStartResponse {
        auth_url,
        state: oauth_state,
    }))
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct OAuthCallbackQuery {
    pub code: Option<String>,
    pub state: Option<String>,
    pub error: Option<String>,
    pub error_description: Option<String>,
}

pub async fn oauth_callback(
    State(state): State<AppState>,
    Path(provider): Path<String>,
    axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
) -> Response {
    if let Some(error) = query.error {
        let description = query.error_description.unwrap_or_default();
        return Html(format!(
            r#"<!DOCTYPE html>
<html>
<head><title>OAuth Error</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>Error: {}</p>
    <p>{}</p>
    <p>You can close this window.</p>
</body>
</html>"#,
            error, description
        ))
        .into_response();
    }

    let oauth_state = match query.state {
        Some(s) => s,
        None => {
            return Html(
                r#"<!DOCTYPE html>
<html>
<head><title>OAuth Error</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>Missing state parameter</p>
    <p>You can close this window.</p>
</body>
</html>"#,
            )
            .into_response();
        }
    };

    let completed = match state.proxy_client.check_oauth_status(&oauth_state).await {
        Ok(c) => c,
        Err(e) => {
            return Html(format!(
                r#"<!DOCTYPE html>
<html>
<head><title>OAuth Error</title></head>
<body>
    <h1>Authentication Failed</h1>
    <p>Failed to check OAuth status: {}</p>
    <p>You can close this window.</p>
</body>
</html>"#,
                e
            ))
            .into_response();
        }
    };

    if !completed {
        return Html(
            r#"<!DOCTYPE html>
<html>
<head><title>OAuth Pending</title></head>
<body>
    <h1>Authentication Pending</h1>
    <p>OAuth flow is still in progress. Please wait...</p>
    <script>setTimeout(() => location.reload(), 2000);</script>
</body>
</html>"#,
        )
        .into_response();
    }

    if let Err(e) = state.proxy_client.sync_provider(&provider).await {
        return Html(format!(
            r#"<!DOCTYPE html>
<html>
<head><title>OAuth Warning</title></head>
<body>
    <h1>Authentication Successful</h1>
    <p>However, failed to sync provider: {}</p>
    <p>You can close this window.</p>
</body>
</html>"#,
            e
        ))
        .into_response();
    }

    Html(format!(
        r#"<!DOCTYPE html>
<html>
<head><title>Success</title></head>
<body>
    <h1>Success!</h1>
    <p>{} has been connected successfully.</p>
    <p>You can close this window.</p>
    <script>window.close();</script>
</body>
</html>"#,
        provider
    ))
    .into_response()
}

// Provider Management (Phase 3.2)

pub async fn list_providers(
    State(state): State<AppState>,
    _session: AdminSession,
) -> Result<Json<ListProvidersResponse>, ProviderError> {
    let providers = state
        .db
        .list_providers()
        .map_err(|e| ProviderError::DatabaseError(e.to_string()))?;

    let mut summaries = Vec::with_capacity(providers.len());
    for provider in providers {
        let accounts_count = state
            .db
            .count_provider_accounts(&provider.name)
            .unwrap_or(0);

        let status = if provider.enabled && accounts_count > 0 {
            "active"
        } else if !provider.enabled {
            "inactive"
        } else {
            "no_accounts"
        };

        summaries.push(ProviderSummary {
            name: provider.name,
            provider_type: provider.provider_type,
            enabled: provider.enabled,
            accounts_count,
            status: status.to_string(),
            created_at: provider.created_at,
            updated_at: provider.updated_at,
        });
    }

    let total = summaries.len();
    Ok(Json(ListProvidersResponse {
        providers: summaries,
        total,
    }))
}

pub async fn get_provider(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(provider): Path<String>,
) -> Result<Json<ProviderSummary>, ProviderError> {
    let db_provider = state
        .db
        .get_provider_by_name(&provider)
        .map_err(|e| ProviderError::DatabaseError(e.to_string()))?
        .ok_or_else(|| ProviderError::NotFound(provider.clone()))?;

    let accounts_count = state
        .db
        .count_provider_accounts(&db_provider.name)
        .unwrap_or(0);

    let status = if db_provider.enabled && accounts_count > 0 {
        "active"
    } else if !db_provider.enabled {
        "inactive"
    } else {
        "no_accounts"
    };

    Ok(Json(ProviderSummary {
        name: db_provider.name,
        provider_type: db_provider.provider_type,
        enabled: db_provider.enabled,
        accounts_count,
        status: status.to_string(),
        created_at: db_provider.created_at,
        updated_at: db_provider.updated_at,
    }))
}

pub async fn get_provider_status(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(provider): Path<String>,
) -> Result<Json<ProviderStatusResponse>, ProviderError> {
    if !is_valid_provider(&provider) {
        return Err(ProviderError::InvalidProvider(provider));
    }

    let status = state
        .proxy_client
        .get_provider_status(&provider)
        .await
        .map_err(|e| ProviderError::ProxyError(e.to_string()))?;

    Ok(Json(ProviderStatusResponse {
        name: status.name,
        status: status.status,
        accounts_count: status.accounts_count,
        last_error: status.last_error,
    }))
}

pub async fn update_provider_settings(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(provider): Path<String>,
    Json(payload): Json<UpdateProviderSettingsRequest>,
) -> Result<Json<ProviderSummary>, ProviderError> {
    let updated = state
        .db
        .update_provider(&provider, None, Some(&payload.settings))
        .map_err(|e| ProviderError::DatabaseError(e.to_string()))?
        .ok_or_else(|| ProviderError::NotFound(provider.clone()))?;

    let accounts_count = state
        .db
        .count_provider_accounts(&updated.name)
        .unwrap_or(0);

    let status = if updated.enabled && accounts_count > 0 {
        "active"
    } else if !updated.enabled {
        "inactive"
    } else {
        "no_accounts"
    };

    Ok(Json(ProviderSummary {
        name: updated.name,
        provider_type: updated.provider_type,
        enabled: updated.enabled,
        accounts_count,
        status: status.to_string(),
        created_at: updated.created_at,
        updated_at: updated.updated_at,
    }))
}

pub async fn delete_provider(
    State(state): State<AppState>,
    _session: AdminSession,
    Path(provider): Path<String>,
) -> Result<Json<SuccessResponse>, ProviderError> {
    let deleted = state
        .db
        .delete_provider(&provider)
        .map_err(|e| ProviderError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(ProviderError::NotFound(provider));
    }

    Ok(Json(SuccessResponse { success: true }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_providers))
        .route("/:provider", get(get_provider).delete(delete_provider))
        .route("/:provider/status", get(get_provider_status))
        .route("/:provider/settings", put(update_provider_settings))
        .route("/:provider/oauth/start", post(start_oauth))
}

pub fn oauth_callback_router() -> Router<AppState> {
    Router::new().route("/:provider/callback", get(oauth_callback))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager, ProxyProviderStatus};
    use crate::db::Database;
    use crate::middleware::rate_limit::RateLimiter;
    use axum::{body::Body, http::Request};
    use base64::{engine::general_purpose::STANDARD, Engine};
    use serde_json::json;
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

    fn create_mock_proxy_client() -> Arc<MockProxyManagementClient> {
        let mock = Arc::new(MockProxyManagementClient::default());
        *mock.oauth_start_response.lock().unwrap() =
            Some(("https://claude.example.com/oauth".to_string(), "test-state-123".to_string()));
        *mock.oauth_status.lock().unwrap() = true;
        *mock.provider_statuses.lock().unwrap() = vec![
            ProxyProviderStatus {
                name: "claude".to_string(),
                status: "healthy".to_string(),
                accounts_count: 0,
                last_error: None,
            },
        ];
        mock
    }

    fn create_app_with_mock(db: Database, mock: Arc<MockProxyManagementClient>) -> Router {
        let state = AppState {
            db,
            rate_limiter: Arc::new(RateLimiter::new(60)),
            proxy_client: mock,
            proxy_manager: Arc::new(MockProxyProcessManager::default()),
        };

        Router::new()
            .nest("/api/providers", router())
            .nest("/oauth", oauth_callback_router())
            .with_state(state)
    }

    fn create_app(db: Database) -> Router {
        create_app_with_mock(db, create_mock_proxy_client())
    }

    fn create_app_with_session(db: Database, session_id: &str) -> (Router, String) {
        db.create_session(session_id, "csrf-token", 7).unwrap();
        (create_app(db), session_id.to_string())
    }

    // OAuth Flow (3.1) Tests

    #[tokio::test]
    async fn test_start_oauth_requires_admin_session() {
        let db = create_test_db();
        let app = create_app(db);

        let request = Request::builder()
            .method("POST")
            .uri("/api/providers/claude/oauth/start")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_start_oauth_returns_auth_url_and_state() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/providers/claude/oauth/start")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: OAuthStartResponse = serde_json::from_slice(&body).unwrap();

        assert!(!json.auth_url.is_empty());
        assert!(json.auth_url.contains("claude"));
        assert!(!json.state.is_empty());
    }

    #[tokio::test]
    async fn test_start_oauth_invalid_provider_returns_400() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("POST")
            .uri("/api/providers/invalid_provider/oauth/start")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn test_oauth_callback_with_valid_state_shows_success() {
        let db = create_test_db();
        let mock = create_mock_proxy_client();
        *mock.oauth_status.lock().unwrap() = true;
        let app = create_app_with_mock(db, mock);

        let request = Request::builder()
            .method("GET")
            .uri("/oauth/claude/callback?code=test_code&state=valid_state")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Success!"));
    }

    #[tokio::test]
    async fn test_oauth_callback_pending_shows_pending_page() {
        let db = create_test_db();
        let mock = create_mock_proxy_client();
        *mock.oauth_status.lock().unwrap() = false;
        let app = create_app_with_mock(db, mock);

        let request = Request::builder()
            .method("GET")
            .uri("/oauth/claude/callback?state=pending_state")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("Pending"));
    }

    #[tokio::test]
    async fn test_oauth_callback_with_error_shows_error_page() {
        let db = create_test_db();
        let app = create_app(db);

        let request = Request::builder()
            .method("GET")
            .uri("/oauth/claude/callback?error=access_denied&error_description=User%20denied%20access")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let body_str = String::from_utf8(body.to_vec()).unwrap();
        assert!(body_str.contains("access_denied"));
        assert!(body_str.contains("User denied access"));
    }

    // Provider Management (3.2) Tests

    #[tokio::test]
    async fn test_list_providers_empty() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/providers")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ListProvidersResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.providers.is_empty());
        assert_eq!(json.total, 0);
    }

    #[tokio::test]
    async fn test_list_providers_with_data() {
        let db = create_test_db();
        db.create_provider("claude", "oauth", true, &json!({}))
            .unwrap();
        db.create_provider("chatgpt", "oauth", false, &json!({}))
            .unwrap();
        db.create_provider_account("claude", "user@example.com", &json!({"token": "test"}))
            .unwrap();

        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/providers")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ListProvidersResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.providers.len(), 2);
        assert_eq!(json.total, 2);

        let claude = json.providers.iter().find(|p| p.name == "claude").unwrap();
        assert_eq!(claude.accounts_count, 1);
        assert_eq!(claude.status, "active");

        let chatgpt = json.providers.iter().find(|p| p.name == "chatgpt").unwrap();
        assert_eq!(chatgpt.accounts_count, 0);
        assert_eq!(chatgpt.status, "inactive");
    }

    #[tokio::test]
    async fn test_list_providers_requires_admin_session() {
        let db = create_test_db();
        let app = create_app(db);

        let request = Request::builder()
            .method("GET")
            .uri("/api/providers")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_get_provider_returns_details() {
        let db = create_test_db();
        db.create_provider("claude", "oauth", true, &json!({"model": "claude-3"}))
            .unwrap();
        db.create_provider_account("claude", "user1@example.com", &json!({"token": "t1"}))
            .unwrap();
        db.create_provider_account("claude", "user2@example.com", &json!({"token": "t2"}))
            .unwrap();

        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/providers/claude")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ProviderSummary = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.name, "claude");
        assert_eq!(json.provider_type, "oauth");
        assert!(json.enabled);
        assert_eq!(json.accounts_count, 2);
        assert_eq!(json.status, "active");
    }

    #[tokio::test]
    async fn test_get_provider_not_found() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/providers/nonexistent")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_get_provider_status() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("GET")
            .uri("/api/providers/claude/status")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ProviderStatusResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.name, "claude");
        assert_eq!(json.status, "healthy");
    }

    #[tokio::test]
    async fn test_update_provider_settings() {
        let db = create_test_db();
        db.create_provider("claude", "oauth", true, &json!({"old": "value"}))
            .unwrap();

        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("PUT")
            .uri("/api/providers/claude/settings")
            .header("Cookie", format!("session={}", session_id))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"settings": {"new": "settings", "key": 123}}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ProviderSummary = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.name, "claude");
    }

    #[tokio::test]
    async fn test_update_provider_settings_not_found() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("PUT")
            .uri("/api/providers/nonexistent/settings")
            .header("Cookie", format!("session={}", session_id))
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"settings": {}}"#))
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_delete_provider_removes_records() {
        let db = create_test_db();
        db.create_provider("claude", "oauth", true, &json!({}))
            .unwrap();

        assert!(db.get_provider_by_name("claude").unwrap().is_some());

        let (app, session_id) = create_app_with_session(db.clone(), "test-session");

        let request = Request::builder()
            .method("DELETE")
            .uri("/api/providers/claude")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: SuccessResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.success);

        assert!(db.get_provider_by_name("claude").unwrap().is_none());
    }

    #[tokio::test]
    async fn test_delete_provider_not_found() {
        let db = create_test_db();
        let (app, session_id) = create_app_with_session(db, "test-session");

        let request = Request::builder()
            .method("DELETE")
            .uri("/api/providers/nonexistent")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }
}
