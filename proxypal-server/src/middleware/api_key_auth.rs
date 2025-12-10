use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{header::AUTHORIZATION, request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum::extract::FromRef;
use serde::Serialize;

use crate::AppState;

#[derive(Debug, Clone)]
pub struct UserContext {
    pub id: i64,
    pub name: String,
    pub quota_tokens: Option<i64>,
    pub used_tokens: i64,
    pub enabled: bool,
}

#[derive(Debug, Serialize)]
pub struct ApiKeyError {
    pub success: bool,
    pub error: String,
    pub code: String,
}

impl ApiKeyError {
    fn unauthorized(message: &str) -> Self {
        Self {
            success: false,
            error: message.to_string(),
            code: "UNAUTHORIZED".to_string(),
        }
    }

    fn forbidden(message: &str) -> Self {
        Self {
            success: false,
            error: message.to_string(),
            code: "FORBIDDEN".to_string(),
        }
    }

    fn quota_exceeded() -> Self {
        Self {
            success: false,
            error: "Quota exceeded".to_string(),
            code: "QUOTA_EXCEEDED".to_string(),
        }
    }
}

pub struct ApiKeyAuth {
    pub user: UserContext,
}

fn extract_prefix(api_key: &str) -> Option<&str> {
    if !api_key.starts_with("sk-") {
        return None;
    }
    let without_sk = &api_key[3..];
    let last_dash = without_sk.rfind('-')?;
    Some(&api_key[..3 + last_dash])
}

#[async_trait]
impl<S> FromRequestParts<S> for ApiKeyAuth
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|v| v.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiKeyError::unauthorized("Missing Authorization header")),
                )
                    .into_response()
            })?;

        let api_key = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError::unauthorized("Invalid Authorization format")),
            )
                .into_response()
        })?;

        if !api_key.starts_with("sk-") {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError::unauthorized("Invalid API key format")),
            )
                .into_response());
        }

        let prefix = extract_prefix(api_key).ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError::unauthorized("Invalid API key format")),
            )
                .into_response()
        })?;

        let user_with_hash = app_state
            .db
            .get_user_by_api_key_prefix(prefix)
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiKeyError::unauthorized("Invalid API key")),
                )
                    .into_response()
            })?
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiKeyError::unauthorized("Invalid API key")),
                )
                    .into_response()
            })?;

        let parsed_hash = PasswordHash::new(&user_with_hash.api_key_hash).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(ApiKeyError::unauthorized("Invalid API key")),
            )
                .into_response()
        })?;

        Argon2::default()
            .verify_password(api_key.as_bytes(), &parsed_hash)
            .map_err(|_| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(ApiKeyError::unauthorized("Invalid API key")),
                )
                    .into_response()
            })?;

        let user = &user_with_hash.user;

        if !user.enabled {
            return Err((
                StatusCode::FORBIDDEN,
                Json(ApiKeyError::forbidden("User is disabled")),
            )
                .into_response());
        }

        if let Some(quota) = user.quota_tokens {
            if user.used_tokens >= quota {
                return Err((
                    StatusCode::TOO_MANY_REQUESTS,
                    Json(ApiKeyError::quota_exceeded()),
                )
                    .into_response());
            }
        }

        Ok(ApiKeyAuth {
            user: UserContext {
                id: user.id,
                name: user.name.clone(),
                quota_tokens: user.quota_tokens,
                used_tokens: user.used_tokens,
                enabled: user.enabled,
            },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    fn create_test_app(db: crate::db::Database) -> Router {
        use std::sync::Arc;
        use crate::middleware::rate_limit::RateLimiter;
        use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
        
        let state = AppState { 
            db, 
            rate_limiter: Arc::new(RateLimiter::new(60)),
            proxy_client: Arc::new(MockProxyManagementClient::default()),
            proxy_manager: Arc::new(MockProxyProcessManager::default()),
        };
        Router::new()
            .route("/protected", get(protected_handler))
            .with_state(state)
    }

    async fn protected_handler(auth: ApiKeyAuth) -> impl IntoResponse {
        Json(serde_json::json!({
            "user_id": auth.user.id,
            "name": auth.user.name
        }))
    }

    #[tokio::test]
    async fn test_missing_authorization_header_returns_401() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let app = create_test_app(db);

        let response = app
            .oneshot(Request::builder().uri("/protected").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "UNAUTHORIZED");
    }

    #[tokio::test]
    async fn test_invalid_format_returns_401() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let app = create_test_app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", "Basic abc123")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_unknown_api_key_returns_401() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let app = create_test_app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", "Bearer sk-unknown-abcd1234abcd1234abcd1234abcd1234")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_wrong_key_for_prefix_returns_401() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let (_, api_key) = db.create_user("testuser", None).unwrap();
        let app = create_test_app(db);

        let wrong_key = format!("sk-testuser-wrongwrongwrongwrongwrong");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {}", wrong_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let _ = api_key;
    }

    #[tokio::test]
    async fn test_valid_key_allows_access() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let (user, api_key) = db.create_user("testuser", None).unwrap();
        let app = create_test_app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["user_id"], user.id);
        assert_eq!(json["name"], "testuser");
    }

    #[tokio::test]
    async fn test_disabled_user_returns_403() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let (user, api_key) = db.create_user("testuser", None).unwrap();
        db.update_user(user.id, None, None, Some(false)).unwrap();
        let app = create_test_app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "FORBIDDEN");
    }

    #[tokio::test]
    async fn test_user_at_quota_returns_429() {
        let db = crate::db::Database::new_in_memory().unwrap();
        let (user, api_key) = db.create_user("testuser", Some(100)).unwrap();
        db.with_conn(|conn| {
            conn.execute("UPDATE users SET used_tokens = 100 WHERE id = ?1", [user.id])?;
            Ok(())
        })
        .unwrap();
        let app = create_test_app(db);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/protected")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "QUOTA_EXCEEDED");
    }
}
