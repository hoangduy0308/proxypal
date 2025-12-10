use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};
use time::{Duration, OffsetDateTime};
use uuid::Uuid;

use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub success: bool,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct LogoutResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct StatusResponse {
    pub authenticated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub success: bool,
    pub error: String,
    pub code: String,
}

#[derive(Debug)]
pub enum AuthError {
    InvalidCredentials,
    NotConfigured,
    DatabaseError(String),
    HashError(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> axum::response::Response {
        let (status, error, code) = match self {
            AuthError::InvalidCredentials => (
                StatusCode::UNAUTHORIZED,
                "Invalid password".to_string(),
                "UNAUTHORIZED".to_string(),
            ),
            AuthError::NotConfigured => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Admin password not configured".to_string(),
                "NOT_CONFIGURED".to_string(),
            ),
            AuthError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
                "INTERNAL_ERROR".to_string(),
            ),
            AuthError::HashError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Authentication error: {}", e),
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

fn create_session_cookie(session_id: &str, expires_at: OffsetDateTime) -> Cookie<'static> {
    Cookie::build(("session", session_id.to_owned()))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .expires(expires_at)
        .build()
}

fn create_csrf_cookie(csrf_token: &str, expires_at: OffsetDateTime) -> Cookie<'static> {
    Cookie::build(("csrf_token", csrf_token.to_owned()))
        .http_only(false)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .expires(expires_at)
        .build()
}

fn clear_cookie(name: &str) -> Cookie<'static> {
    Cookie::build((name.to_owned(), ""))
        .http_only(true)
        .secure(true)
        .same_site(SameSite::Strict)
        .path("/")
        .max_age(Duration::ZERO)
        .build()
}

const SESSION_TTL_DAYS: i64 = 7;

pub async fn login(
    State(state): State<AppState>,
    jar: CookieJar,
    Json(payload): Json<LoginRequest>,
) -> Result<(CookieJar, Json<LoginResponse>), AuthError> {
    let admin_password_hash = state
        .db
        .get_setting("admin_password_hash")
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?
        .ok_or(AuthError::NotConfigured)?;

    let parsed_hash = PasswordHash::new(&admin_password_hash)
        .map_err(|e| AuthError::HashError(e.to_string()))?;

    Argon2::default()
        .verify_password(payload.password.as_bytes(), &parsed_hash)
        .map_err(|_| AuthError::InvalidCredentials)?;

    let session_id = Uuid::new_v4().to_string();
    let csrf_token = Uuid::new_v4().to_string();

    let session = state
        .db
        .create_session(&session_id, &csrf_token, SESSION_TTL_DAYS)
        .map_err(|e| AuthError::DatabaseError(e.to_string()))?;

    let expires_at = OffsetDateTime::now_utc() + Duration::days(SESSION_TTL_DAYS);

    let jar = jar
        .add(create_session_cookie(&session_id, expires_at))
        .add(create_csrf_cookie(&csrf_token, expires_at));

    Ok((
        jar,
        Json(LoginResponse {
            success: true,
            message: "Logged in successfully".to_string(),
            expires_at: Some(session.expires_at),
        }),
    ))
}

pub async fn logout(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<(CookieJar, Json<LogoutResponse>), AuthError> {
    if let Some(session_cookie) = jar.get("session") {
        let session_id = session_cookie.value();
        let _ = state.db.delete_session(session_id);
    }

    let jar = jar
        .add(clear_cookie("session"))
        .add(clear_cookie("csrf_token"));

    Ok((
        jar,
        Json(LogoutResponse { success: true }),
    ))
}

pub async fn status(
    State(state): State<AppState>,
    jar: CookieJar,
) -> Result<Json<StatusResponse>, AuthError> {
    let session_cookie = match jar.get("session") {
        Some(cookie) => cookie,
        None => {
            return Ok(Json(StatusResponse {
                authenticated: false,
                expires_at: None,
            }));
        }
    };

    let session_id = session_cookie.value();

    match state.db.get_session(session_id) {
        Ok(Some(session)) => {
            Ok(Json(StatusResponse {
                authenticated: true,
                expires_at: Some(session.expires_at),
            }))
        }
        Ok(None) => Ok(Json(StatusResponse {
            authenticated: false,
            expires_at: None,
        })),
        Err(e) => Err(AuthError::DatabaseError(e.to_string())),
    }
}

pub fn router() -> axum::Router<AppState> {
    use axum::routing::{get, post};
    
    axum::Router::new()
        .route("/login", post(login))
        .route("/logout", post(logout))
        .route("/status", get(status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use axum::{body::Body, http::Request};
    use rand::rngs::OsRng;
    use tempfile::tempdir;
    use tower::ServiceExt;
    
    fn create_test_db() -> Database {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path).unwrap();
        db
    }
    
    fn hash_password(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .unwrap()
            .to_string()
    }
    
    fn create_app(db: Database) -> axum::Router {
        use std::sync::Arc;
        use crate::middleware::rate_limit::RateLimiter;
        use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
        
        let state = AppState { 
            db, 
            rate_limiter: Arc::new(RateLimiter::new(60)),
            proxy_client: Arc::new(MockProxyManagementClient::default()),
            proxy_manager: Arc::new(MockProxyProcessManager::default()),
        };
        axum::Router::new()
            .nest("/api/auth", router())
            .with_state(state)
    }

    #[tokio::test]
    async fn test_login_without_password_configured_returns_error() {
        let db = create_test_db();
        let app = create_app(db);
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"password":"test"}"#))
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    async fn test_login_with_invalid_password_returns_401() {
        let db = create_test_db();
        let hash = hash_password("correct_password");
        db.set_setting("admin_password_hash", &hash).unwrap();
        
        let app = create_app(db);
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"password":"wrong_password"}"#))
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_login_with_valid_password_returns_200_and_cookies() {
        let db = create_test_db();
        let hash = hash_password("correct_password");
        db.set_setting("admin_password_hash", &hash).unwrap();
        
        let app = create_app(db);
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/login")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"password":"correct_password"}"#))
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let cookies: Vec<_> = response
            .headers()
            .get_all("set-cookie")
            .iter()
            .collect();
        assert!(cookies.len() >= 2, "Should set session and csrf_token cookies");
        
        let cookie_strs: Vec<String> = cookies
            .iter()
            .map(|c| c.to_str().unwrap().to_string())
            .collect();
        assert!(cookie_strs.iter().any(|c| c.starts_with("session=")));
        assert!(cookie_strs.iter().any(|c| c.starts_with("csrf_token=")));
    }

    #[tokio::test]
    async fn test_status_without_session_returns_unauthenticated() {
        let db = create_test_db();
        let app = create_app(db);
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/auth/status")
            .body(Body::empty())
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(!json.authenticated);
    }

    #[tokio::test]
    async fn test_status_with_valid_session_returns_authenticated() {
        let db = create_test_db();
        let session_id = "test-session-id";
        let csrf_token = "test-csrf-token";
        db.create_session(session_id, csrf_token, 7).unwrap();
        
        let app = create_app(db);
        
        let request = Request::builder()
            .method("GET")
            .uri("/api/auth/status")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        let body = axum::body::to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let json: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(json.authenticated);
        assert!(json.expires_at.is_some());
    }

    #[tokio::test]
    async fn test_logout_clears_session() {
        let db = create_test_db();
        let session_id = "test-session-id";
        let csrf_token = "test-csrf-token";
        db.create_session(session_id, csrf_token, 7).unwrap();
        
        let app = create_app(db.clone());
        
        let request = Request::builder()
            .method("POST")
            .uri("/api/auth/logout")
            .header("Cookie", format!("session={}", session_id))
            .body(Body::empty())
            .unwrap();
            
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        
        // Verify session was deleted
        let session = db.get_session(session_id).unwrap();
        assert!(session.is_none());
    }
}
