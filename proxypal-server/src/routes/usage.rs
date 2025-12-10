use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::db::usage::{DailyUsage, UsageLog};
use crate::middleware::admin_auth::AdminSession;
use crate::AppState;

#[derive(Debug)]
pub enum UsageError {
    UserNotFound,
    DatabaseError(String),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub code: String,
}

impl IntoResponse for UsageError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, code) = match self {
            UsageError::UserNotFound => (
                StatusCode::NOT_FOUND,
                "User not found".to_string(),
                "NOT_FOUND".to_string(),
            ),
            UsageError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
                "INTERNAL_ERROR".to_string(),
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

#[derive(Debug, Deserialize)]
pub struct UsageQuery {
    period: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageResponse {
    period: String,
    total_requests: i64,
    total_tokens_input: i64,
    total_tokens_output: i64,
    by_provider: HashMap<String, ProviderUsageResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsageResponse {
    requests: i64,
    tokens_input: i64,
    tokens_output: i64,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserUsageResponse {
    user_id: i64,
    user_name: String,
    period: String,
    total_requests: i64,
    total_tokens_input: i64,
    total_tokens_output: i64,
    by_provider: HashMap<String, ProviderUsageResponse>,
}

#[derive(Debug, Deserialize)]
pub struct DailyUsageQuery {
    days: Option<u32>,
    user_id: Option<i64>,
    provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsageResponse {
    days: u32,
    data: Vec<DailyUsage>,
}

#[derive(Debug, Deserialize)]
pub struct LogsQuery {
    limit: Option<u32>,
    offset: Option<u32>,
    user_id: Option<i64>,
    provider: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsResponse {
    logs: Vec<UsageLog>,
    total: u64,
    limit: u32,
    offset: u32,
}

pub async fn get_usage(
    _session: AdminSession,
    State(state): State<AppState>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UsageResponse>, UsageError> {
    let period = query.period.unwrap_or_else(|| "month".to_string());

    let stats = state
        .db
        .get_usage_stats(&period)
        .map_err(|e| UsageError::DatabaseError(e.to_string()))?;

    let provider_usage = state
        .db
        .get_usage_by_provider(&period)
        .map_err(|e| UsageError::DatabaseError(e.to_string()))?;

    let by_provider: HashMap<String, ProviderUsageResponse> = provider_usage
        .into_iter()
        .map(|p| {
            (
                p.provider,
                ProviderUsageResponse {
                    requests: p.requests,
                    tokens_input: p.tokens_input,
                    tokens_output: p.tokens_output,
                },
            )
        })
        .collect();

    Ok(Json(UsageResponse {
        period,
        total_requests: stats.total_requests,
        total_tokens_input: stats.total_tokens_input,
        total_tokens_output: stats.total_tokens_output,
        by_provider,
    }))
}

pub async fn get_user_usage(
    _session: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(query): Query<UsageQuery>,
) -> Result<Json<UserUsageResponse>, UsageError> {
    let period = query.period.unwrap_or_else(|| "month".to_string());

    let user = state
        .db
        .get_user_by_id(id)
        .map_err(|e| UsageError::DatabaseError(e.to_string()))?
        .ok_or(UsageError::UserNotFound)?;

    let stats = state
        .db
        .get_user_usage(id, &period)
        .map_err(|e| UsageError::DatabaseError(e.to_string()))?;

    let by_provider = HashMap::new();

    Ok(Json(UserUsageResponse {
        user_id: id,
        user_name: user.name,
        period,
        total_requests: stats.total_requests,
        total_tokens_input: stats.total_tokens_input,
        total_tokens_output: stats.total_tokens_output,
        by_provider,
    }))
}

pub async fn get_daily_usage(
    _session: AdminSession,
    State(state): State<AppState>,
    Query(query): Query<DailyUsageQuery>,
) -> Result<Json<DailyUsageResponse>, UsageError> {
    let days = query.days.unwrap_or(30).min(90);

    let data = state
        .db
        .get_daily_usage(days, query.user_id, query.provider.as_deref())
        .map_err(|e| UsageError::DatabaseError(e.to_string()))?;

    Ok(Json(DailyUsageResponse { days, data }))
}

pub async fn get_logs(
    _session: AdminSession,
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, UsageError> {
    let limit = query.limit.unwrap_or(100).min(1000);
    let offset = query.offset.unwrap_or(0);

    let (logs, total) = state
        .db
        .get_usage_logs_paginated(limit, offset, query.user_id, query.provider.as_deref())
        .map_err(|e| UsageError::DatabaseError(e.to_string()))?;

    Ok(Json(LogsResponse {
        logs,
        total,
        limit,
        offset,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(get_usage))
        .route("/users/:id", get(get_user_usage))
        .route("/daily", get(get_daily_usage))
        .route("/logs", get(get_logs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use axum::{body::Body, http::Request};
    use tempfile::{tempdir, TempDir};
    use tower::ServiceExt;

    fn create_test_db() -> (Database, TempDir) {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path).unwrap();
        (db, dir)
    }

    fn create_app(db: Database) -> (Router, String) {
        use std::sync::Arc;
        use crate::middleware::rate_limit::RateLimiter;
        use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
        
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
            .nest("/api/usage", router())
            .with_state(state);

        (app, session_id.to_string())
    }

    fn authed_request(method: &str, uri: &str, session_id: &str) -> Request<Body> {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn test_get_usage_returns_empty_stats_initially() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/usage", &session_id);
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: UsageResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.period, "month");
        assert_eq!(json.total_requests, 0);
        assert_eq!(json.total_tokens_input, 0);
        assert_eq!(json.total_tokens_output, 0);
        assert!(json.by_provider.is_empty());
    }

    #[tokio::test]
    async fn test_get_daily_usage_returns_empty_array_initially() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/usage/daily", &session_id);
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: DailyUsageResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.days, 30);
        assert!(json.data.is_empty());
    }

    #[tokio::test]
    async fn test_get_logs_returns_empty_list_initially() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/usage/logs", &session_id);
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: LogsResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.logs.is_empty());
        assert_eq!(json.total, 0);
        assert_eq!(json.limit, 100);
        assert_eq!(json.offset, 0);
    }

    #[tokio::test]
    async fn test_get_user_usage_returns_stats_for_specific_user() {
        let (db, _dir) = create_test_db();
        let (user, _) = db.create_user("testuser", None).unwrap();

        let (app, session_id) = create_app(db);

        let request = authed_request(
            "GET",
            &format!("/api/usage/users/{}", user.id),
            &session_id,
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: UserUsageResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.user_id, user.id);
        assert_eq!(json.user_name, "testuser");
        assert_eq!(json.total_requests, 0);
    }

    #[tokio::test]
    async fn test_get_user_usage_returns_404_for_nonexistent_user() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/usage/users/99999", &session_id);
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_unauthenticated_request_returns_401() {
        use std::sync::Arc;
        use crate::middleware::rate_limit::RateLimiter;
        use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
        
        let (db, _dir) = create_test_db();
        let state = AppState { 
            db, 
            rate_limiter: Arc::new(RateLimiter::new(60)),
            proxy_client: Arc::new(MockProxyManagementClient::default()),
            proxy_manager: Arc::new(MockProxyProcessManager::default()),
        };
        let app = Router::new()
            .nest("/api/usage", router())
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/usage")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
