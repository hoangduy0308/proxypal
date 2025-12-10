use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::db::users::User;
use crate::middleware::admin_auth::AdminSession;
use crate::AppState;

#[derive(Debug)]
pub enum UserError {
    NotFound,
    Conflict(String),
    DatabaseError(String),
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub code: String,
}

impl IntoResponse for UserError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, code) = match self {
            UserError::NotFound => (
                StatusCode::NOT_FOUND,
                "User not found".to_string(),
                "NOT_FOUND".to_string(),
            ),
            UserError::Conflict(msg) => (StatusCode::CONFLICT, msg, "CONFLICT".to_string()),
            UserError::DatabaseError(e) => (
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
pub struct ListUsersQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ListUsersResponse {
    users: Vec<User>,
    total: u64,
    page: u32,
    limit: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserRequest {
    name: String,
    quota_tokens: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateUserResponse {
    #[serde(flatten)]
    user: User,
    api_key: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateUserRequest {
    name: Option<String>,
    quota_tokens: Option<Option<i64>>,
    enabled: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegenerateKeyResponse {
    #[serde(flatten)]
    user: User,
    api_key: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResetUsageResponse {
    success: bool,
    previous_used_tokens: i64,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeleteResponse {
    success: bool,
}

pub async fn list_users(
    _session: AdminSession,
    State(state): State<AppState>,
    Query(query): Query<ListUsersQuery>,
) -> Result<Json<ListUsersResponse>, UserError> {
    let page = query.page.unwrap_or(1).max(1);
    let limit = query.limit.unwrap_or(20).clamp(1, 100);

    let (users, total) = state
        .db
        .list_users_paginated(page, limit)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

    Ok(Json(ListUsersResponse {
        users,
        total,
        page,
        limit,
    }))
}

pub async fn create_user(
    _session: AdminSession,
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<(StatusCode, Json<CreateUserResponse>), UserError> {
    let (user, api_key) = state
        .db
        .create_user(&payload.name, payload.quota_tokens)
        .map_err(|e| {
            let msg = e.to_string();
            if msg.contains("already exists") {
                UserError::Conflict(msg)
            } else {
                UserError::DatabaseError(msg)
            }
        })?;

    Ok((StatusCode::CREATED, Json(CreateUserResponse { user, api_key })))
}

pub async fn get_user(
    _session: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<User>, UserError> {
    let user = state
        .db
        .get_user_by_id(id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::NotFound)?;

    Ok(Json(user))
}

pub async fn update_user(
    _session: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<UpdateUserRequest>,
) -> Result<Json<User>, UserError> {
    let user = state
        .db
        .update_user(
            id,
            payload.name.as_deref(),
            payload.quota_tokens,
            payload.enabled,
        )
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::NotFound)?;

    Ok(Json(user))
}

pub async fn delete_user(
    _session: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DeleteResponse>, UserError> {
    let deleted = state
        .db
        .delete_user(id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?;

    if !deleted {
        return Err(UserError::NotFound);
    }

    Ok(Json(DeleteResponse { success: true }))
}

pub async fn regenerate_key(
    _session: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<RegenerateKeyResponse>, UserError> {
    let (user, api_key) = state
        .db
        .regenerate_api_key(id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::NotFound)?;

    Ok(Json(RegenerateKeyResponse { user, api_key }))
}

pub async fn reset_usage(
    _session: AdminSession,
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ResetUsageResponse>, UserError> {
    let previous_used_tokens = state
        .db
        .reset_used_tokens(id)
        .map_err(|e| UserError::DatabaseError(e.to_string()))?
        .ok_or(UserError::NotFound)?;

    Ok(Json(ResetUsageResponse {
        success: true,
        previous_used_tokens,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_users).post(create_user))
        .route("/:id", get(get_user).put(update_user).delete(delete_user))
        .route("/:id/regenerate-key", post(regenerate_key))
        .route("/:id/reset-usage", post(reset_usage))
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
            .nest("/api/users", router())
            .with_state(state);

        (app, session_id.to_string())
    }

    fn authed_request(method: &str, uri: &str, session_id: &str, body: Option<&str>) -> Request<Body> {
        let mut builder = Request::builder()
            .method(method)
            .uri(uri)
            .header("Cookie", format!("session={}", session_id));

        if body.is_some() {
            builder = builder.header("Content-Type", "application/json");
        }

        builder
            .body(body.map(|b| Body::from(b.to_string())).unwrap_or(Body::empty()))
            .unwrap()
    }

    #[tokio::test]
    async fn test_list_users_returns_empty_list_initially() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/users", &session_id, None);
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: ListUsersResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.users.is_empty());
        assert_eq!(json.total, 0);
        assert_eq!(json.page, 1);
    }

    #[tokio::test]
    async fn test_create_user_returns_user_with_api_key() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request(
            "POST",
            "/api/users",
            &session_id,
            Some(r#"{"name":"testuser"}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: CreateUserResponse = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.user.name, "testuser");
        assert!(json.api_key.starts_with("sk-testuser-"));
        assert!(json.user.enabled);
    }

    #[tokio::test]
    async fn test_get_user_by_id_returns_user() {
        let (db, _dir) = create_test_db();
        let (user, _) = db.create_user("testuser", None).unwrap();

        let (app, session_id) = create_app(db);

        let request = authed_request(
            "GET",
            &format!("/api/users/{}", user.id),
            &session_id,
            None,
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.id, user.id);
        assert_eq!(json.name, "testuser");
    }

    #[tokio::test]
    async fn test_update_user_changes_fields() {
        let (db, _dir) = create_test_db();
        let (user, _) = db.create_user("testuser", None).unwrap();

        let (app, session_id) = create_app(db);

        let request = authed_request(
            "PUT",
            &format!("/api/users/{}", user.id),
            &session_id,
            Some(r#"{"name":"updated","enabled":false}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: User = serde_json::from_slice(&body).unwrap();

        assert_eq!(json.name, "updated");
        assert!(!json.enabled);
    }

    #[tokio::test]
    async fn test_delete_user_removes_user() {
        let (db, _dir) = create_test_db();
        let (user, _) = db.create_user("testuser", None).unwrap();

        let (app, session_id) = create_app(db.clone());

        let request = authed_request(
            "DELETE",
            &format!("/api/users/{}", user.id),
            &session_id,
            None,
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let deleted_user = db.get_user_by_id(user.id).unwrap();
        assert!(deleted_user.is_none());
    }

    #[tokio::test]
    async fn test_regenerate_key_returns_new_key() {
        let (db, _dir) = create_test_db();
        let (user, original_key) = db.create_user("testuser", None).unwrap();

        let (app, session_id) = create_app(db);

        let request = authed_request(
            "POST",
            &format!("/api/users/{}/regenerate-key", user.id),
            &session_id,
            None,
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: RegenerateKeyResponse = serde_json::from_slice(&body).unwrap();

        assert!(json.api_key.starts_with("sk-testuser-"));
        assert_ne!(json.api_key, original_key);
    }

    #[tokio::test]
    async fn test_create_duplicate_user_returns_409() {
        let (db, _dir) = create_test_db();
        db.create_user("testuser", None).unwrap();

        let (app, session_id) = create_app(db);

        let request = authed_request(
            "POST",
            "/api/users",
            &session_id,
            Some(r#"{"name":"testuser"}"#),
        );
        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn test_get_nonexistent_user_returns_404() {
        let (db, _dir) = create_test_db();
        let (app, session_id) = create_app(db);

        let request = authed_request("GET", "/api/users/99999", &session_id, None);
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
            .nest("/api/users", router())
            .with_state(state);

        let request = Request::builder()
            .method("GET")
            .uri("/api/users")
            .body(Body::empty())
            .unwrap();

        let response = app.oneshot(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
