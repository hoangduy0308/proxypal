//! End-to-End Integration Tests for ProxyPal Server
//!
//! Tests the full API flow using axum-test TestServer to simulate
//! real HTTP requests across the entire application.

use axum::http::{header::HeaderName, HeaderValue};
use axum_test::TestServer;
use base64::{engine::general_purpose::STANDARD, Engine};
use proxypal_server::{
    cliproxy::{MockProxyManagementClient, MockProxyProcessManager, ProxyProviderStatus},
    db::Database,
    middleware::rate_limit::RateLimiter,
    routes, AppState,
};
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::Arc;
use tempfile::tempdir;

// =============================================================================
// Test Setup Utilities
// =============================================================================

fn setup_test_env() {
    let key = [0u8; 32];
    std::env::set_var("ENCRYPTION_KEY", STANDARD.encode(key));
}

fn cleanup_env() {
    std::env::remove_var("ENCRYPTION_KEY");
}

fn hash_password(password: &str) -> String {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .unwrap()
        .to_string()
}

fn create_test_db() -> Database {
    setup_test_env();
    let dir = tempdir().unwrap();
    let path = dir.path().join("test.db");
    // Keep the tempdir alive by leaking it (acceptable for tests)
    std::mem::forget(dir);
    Database::new(path).unwrap()
}

fn create_test_state() -> AppState {
    let db = create_test_db();
    let rate_limiter = Arc::new(RateLimiter::new(60));
    let proxy_client: Arc<dyn proxypal_server::cliproxy::ProxyManagementClient> =
        Arc::new(create_mock_proxy_client());
    let proxy_manager: Arc<dyn proxypal_server::cliproxy::ProxyProcessManager> =
        Arc::new(MockProxyProcessManager::default());

    AppState {
        db,
        rate_limiter,
        proxy_client,
        proxy_manager,
    }
}

fn create_mock_proxy_client() -> MockProxyManagementClient {
    let mock = MockProxyManagementClient::default();
    *mock.oauth_start_response.lock().unwrap() = Some((
        "https://claude.example.com/oauth".to_string(),
        "test-state-123".to_string(),
    ));
    *mock.oauth_status.lock().unwrap() = true;
    *mock.provider_statuses.lock().unwrap() = vec![ProxyProviderStatus {
        name: "claude".to_string(),
        status: "healthy".to_string(),
        accounts_count: 0,
        last_error: None,
    }];
    mock
}

fn create_full_app(state: AppState) -> axum::Router {
    use axum::{routing::get, Router};

    let admin_api = Router::new()
        .nest("/auth", routes::auth::router())
        .nest("/users", routes::users::router())
        .nest("/usage", routes::usage::router())
        .nest("/providers", routes::providers::router())
        .nest("/proxy", routes::proxy::router())
        .nest("/config", routes::config::router());

    Router::new()
        .route("/healthz", get(health_check))
        .nest("/api", admin_api)
        .nest("/oauth", routes::providers::oauth_callback_router())
        .with_state(state)
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> axum::response::Json<Value> {
    let proxy_running = state.proxy_manager.is_running();
    let database_connected = state.db.conn().is_ok();

    axum::response::Json(json!({
        "status": if database_connected && proxy_running { "ok" } else if database_connected { "degraded" } else { "error" },
        "proxy_running": proxy_running,
        "database_connected": database_connected,
    }))
}

async fn create_test_server() -> TestServer {
    let state = create_test_state();
    let app = create_full_app(state);
    TestServer::new(app).unwrap()
}

async fn create_test_server_with_admin(password: &str) -> TestServer {
    let state = create_test_state();
    let hash = hash_password(password);
    state.db.set_setting("admin_password_hash", &hash).unwrap();
    let app = create_full_app(state);
    TestServer::new(app).unwrap()
}

fn cookie_header() -> HeaderName {
    HeaderName::from_static("cookie")
}

fn extract_session_cookie(response: &axum_test::TestResponse) -> String {
    let cookies: Vec<_> = response.iter_headers_by_name("set-cookie").collect();
    for cookie in cookies {
        let cookie_str = cookie.to_str().unwrap();
        if cookie_str.starts_with("session=") {
            return cookie_str.split(';').next().unwrap().to_string();
        }
    }
    panic!("No session cookie found");
}

// =============================================================================
// Scenario 1: Admin Login and Provider Setup
// =============================================================================

mod admin_auth_flow {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_login_with_correct_password_returns_200_and_cookies() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let response = server
            .post("/api/auth/login")
            .json(&json!({"password": "admin123"}))
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert_eq!(body["message"], "Logged in successfully");

        let cookies: Vec<_> = response.iter_headers_by_name("set-cookie").collect();
        assert!(cookies.len() >= 2, "Should set session and csrf_token cookies");

        let cookie_strs: Vec<String> = cookies
            .iter()
            .map(|c| c.to_str().unwrap().to_string())
            .collect();
        assert!(cookie_strs.iter().any(|c| c.starts_with("session=")));
        assert!(cookie_strs.iter().any(|c| c.starts_with("csrf_token=")));

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_login_with_wrong_password_returns_401() {
        cleanup_env();
        let server = create_test_server_with_admin("correct_password").await;

        let response = server
            .post("/api/auth/login")
            .json(&json!({"password": "wrong_password"}))
            .await;

        response.assert_status_unauthorized();

        let body: Value = response.json();
        assert!(!body["success"].as_bool().unwrap());
        assert_eq!(body["code"], "UNAUTHORIZED");

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_providers_list_empty_initially() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": "admin123"}))
            .await;

        let session_cookie = extract_session_cookie(&login_response);

        let response = server
            .get("/api/providers")
            .add_header(cookie_header(), HeaderValue::from_str(&session_cookie).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["providers"].as_array().unwrap().is_empty());
        assert_eq!(body["total"], 0);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_session_cookie_works_for_subsequent_requests() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": "admin123"}))
            .await;

        let session_cookie = extract_session_cookie(&login_response);

        let status_response = server
            .get("/api/auth/status")
            .add_header(cookie_header(), HeaderValue::from_str(&session_cookie).unwrap())
            .await;

        status_response.assert_status_ok();

        let body: Value = status_response.json();
        assert!(body["authenticated"].as_bool().unwrap());
        assert!(body["expires_at"].is_string());

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_unauthenticated_request_to_protected_route_returns_401() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let response = server.get("/api/providers").await;

        response.assert_status_unauthorized();

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_logout_clears_session() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": "admin123"}))
            .await;

        let session_cookie = extract_session_cookie(&login_response);

        let logout_response = server
            .post("/api/auth/logout")
            .add_header(cookie_header(), HeaderValue::from_str(&session_cookie).unwrap())
            .await;

        logout_response.assert_status_ok();

        let protected_response = server
            .get("/api/providers")
            .add_header(cookie_header(), HeaderValue::from_str(&session_cookie).unwrap())
            .await;

        protected_response.assert_status_unauthorized();

        cleanup_env();
    }
}

// =============================================================================
// Scenario 2: User Creation and API Key Management
// =============================================================================

mod user_management_flow {
    use super::*;

    async fn login_and_get_session(server: &TestServer, password: &str) -> String {
        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": password}))
            .await;

        extract_session_cookie(&login_response)
    }

    #[tokio::test]
    #[serial]
    async fn test_create_user_returns_201_with_api_key() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser", "quotaTokens": 1000000}))
            .await;

        response.assert_status(axum::http::StatusCode::CREATED);

        let body: Value = response.json();
        assert_eq!(body["name"], "testuser");
        assert!(body["apiKey"].as_str().unwrap().starts_with("sk-testuser-"));
        assert!(body["enabled"].as_bool().unwrap());
        assert_eq!(body["quotaTokens"], 1000000);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_list_users_shows_created_user() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "user1"}))
            .await;

        server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "user2", "quotaTokens": 500000}))
            .await;

        let response = server
            .get("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        let users = body["users"].as_array().unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(body["total"], 2);

        let user1 = users.iter().find(|u| u["name"] == "user1").unwrap();
        assert!(user1["quotaTokens"].is_null());

        let user2 = users.iter().find(|u| u["name"] == "user2").unwrap();
        assert_eq!(user2["quotaTokens"], 500000);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_regenerate_key_returns_new_key() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let create_response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser"}))
            .await;

        let created: Value = create_response.json();
        let user_id = created["id"].as_i64().unwrap();
        let original_key = created["apiKey"].as_str().unwrap().to_string();

        let response = server
            .post(&format!("/api/users/{}/regenerate-key", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        let new_key = body["apiKey"].as_str().unwrap();
        assert!(new_key.starts_with("sk-testuser-"));
        assert_ne!(new_key, original_key);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_update_user_quota_and_enabled() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let create_response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser"}))
            .await;

        let created: Value = create_response.json();
        let user_id = created["id"].as_i64().unwrap();

        let update_response = server
            .put(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"quotaTokens": 2000000, "enabled": false}))
            .await;

        update_response.assert_status_ok();

        let body: Value = update_response.json();
        assert_eq!(body["quotaTokens"], 2000000);
        assert!(!body["enabled"].as_bool().unwrap());

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_user_removes_user() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let create_response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser"}))
            .await;

        let created: Value = create_response.json();
        let user_id = created["id"].as_i64().unwrap();

        let delete_response = server
            .delete(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        delete_response.assert_status_ok();

        let body: Value = delete_response.json();
        assert!(body["success"].as_bool().unwrap());

        let get_response = server
            .get(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        get_response.assert_status_not_found();

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_create_duplicate_user_returns_409() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser"}))
            .await;

        let response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser"}))
            .await;

        response.assert_status(axum::http::StatusCode::CONFLICT);

        cleanup_env();
    }
}

// =============================================================================
// Scenario 3: Usage Tracking
// =============================================================================

mod usage_tracking_flow {
    use super::*;

    async fn login_and_get_session(server: &TestServer, password: &str) -> String {
        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": password}))
            .await;

        extract_session_cookie(&login_response)
    }

    #[tokio::test]
    #[serial]
    async fn test_usage_returns_empty_stats_initially() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .get("/api/usage")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert_eq!(body["period"], "month");
        assert_eq!(body["totalRequests"], 0);
        assert_eq!(body["totalTokensInput"], 0);
        assert_eq!(body["totalTokensOutput"], 0);
        assert!(body["byProvider"].as_object().unwrap().is_empty());

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_user_usage_returns_stats_for_specific_user() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let create_response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "testuser"}))
            .await;

        let created: Value = create_response.json();
        let user_id = created["id"].as_i64().unwrap();

        let response = server
            .get(&format!("/api/usage/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert_eq!(body["userId"], user_id);
        assert_eq!(body["userName"], "testuser");
        assert_eq!(body["totalRequests"], 0);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_user_usage_returns_404_for_nonexistent_user() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .get("/api/usage/users/99999")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_not_found();

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_daily_usage_returns_empty_initially() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .get("/api/usage/daily")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert_eq!(body["days"], 30);
        assert!(body["data"].as_array().unwrap().is_empty());

        cleanup_env();
    }
}

// =============================================================================
// Scenario 4: Proxy Management
// =============================================================================

mod proxy_management_flow {
    use super::*;

    async fn login_and_get_session(server: &TestServer, password: &str) -> String {
        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": password}))
            .await;

        extract_session_cookie(&login_response)
    }

    #[tokio::test]
    #[serial]
    async fn test_proxy_status_when_not_running() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .get("/api/proxy/status")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(!body["running"].as_bool().unwrap());
        assert!(body["pid"].is_null());
        assert_eq!(body["port"], 8317);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_start_proxy_returns_pid() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .post("/api/proxy/start")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["pid"].is_number());
        assert_eq!(body["port"], 8317);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_stop_proxy_succeeds() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .post("/api/proxy/stop")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["success"].as_bool().unwrap());

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_restart_proxy_returns_new_pid() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .post("/api/proxy/restart")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["pid"].is_number());
        assert_eq!(body["port"], 8317);

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_proxy_requires_admin_session() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let response = server.get("/api/proxy/status").await;
        response.assert_status_unauthorized();

        let response = server.post("/api/proxy/start").await;
        response.assert_status_unauthorized();

        let response = server.post("/api/proxy/stop").await;
        response.assert_status_unauthorized();

        let response = server.post("/api/proxy/restart").await;
        response.assert_status_unauthorized();

        cleanup_env();
    }
}

// =============================================================================
// Scenario 5: Config Management
// =============================================================================

mod config_management_flow {
    use super::*;

    async fn login_and_get_session(server: &TestServer, password: &str) -> String {
        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": password}))
            .await;

        extract_session_cookie(&login_response)
    }

    #[tokio::test]
    #[serial]
    async fn test_get_config_returns_defaults() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .get("/api/config")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert_eq!(body["proxy_port"], 8317);
        assert_eq!(body["admin_port"], 3000);
        assert_eq!(body["log_level"], "info");

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_update_config_persists_changes() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let update_response = server
            .put("/api/config")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"proxy_port": 9000, "log_level": "debug"}))
            .await;

        update_response.assert_status_ok();

        let body: Value = update_response.json();
        assert!(body["success"].as_bool().unwrap());

        let get_response = server
            .get("/api/config")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        let config: Value = get_response.json();
        assert_eq!(config["proxy_port"], 9000);
        assert_eq!(config["log_level"], "debug");

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_update_config_admin_port_requires_restart() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .put("/api/config")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"admin_port": 4000}))
            .await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["success"].as_bool().unwrap());
        assert!(body["restart_required"].as_bool().unwrap());

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_update_config_rejects_invalid_port() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .put("/api/config")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"proxy_port": 80}))
            .await;

        response.assert_status_bad_request();

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_update_config_rejects_invalid_log_level() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        let response = server
            .put("/api/config")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"log_level": "invalid"}))
            .await;

        response.assert_status_bad_request();

        cleanup_env();
    }

    #[tokio::test]
    #[serial]
    async fn test_config_requires_admin_session() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;

        let response = server.get("/api/config").await;
        response.assert_status_unauthorized();

        let response = server.put("/api/config").json(&json!({})).await;
        response.assert_status_unauthorized();

        cleanup_env();
    }
}

// =============================================================================
// Scenario 6: Full User Lifecycle Flow
// =============================================================================

mod full_user_lifecycle {
    use super::*;

    async fn login_and_get_session(server: &TestServer, password: &str) -> String {
        let login_response = server
            .post("/api/auth/login")
            .json(&json!({"password": password}))
            .await;

        extract_session_cookie(&login_response)
    }

    #[tokio::test]
    #[serial]
    async fn test_complete_user_lifecycle() {
        cleanup_env();
        let server = create_test_server_with_admin("admin123").await;
        let session = login_and_get_session(&server, "admin123").await;

        // Step 1: Create user with quota
        let create_response = server
            .post("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"name": "lifecycle_user", "quotaTokens": 1000000}))
            .await;

        create_response.assert_status(axum::http::StatusCode::CREATED);
        let created: Value = create_response.json();
        let user_id = created["id"].as_i64().unwrap();
        let original_key = created["apiKey"].as_str().unwrap().to_string();
        assert!(original_key.starts_with("sk-lifecycle_user-"));

        // Step 2: Verify user appears in list
        let list_response = server
            .get("/api/users")
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        let users: Value = list_response.json();
        assert_eq!(users["total"], 1);

        // Step 3: Get user by ID
        let get_response = server
            .get(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        get_response.assert_status_ok();
        let user: Value = get_response.json();
        assert_eq!(user["name"], "lifecycle_user");

        // Step 4: Update user quota and disable
        let update_response = server
            .put(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"quotaTokens": 2000000, "enabled": false}))
            .await;

        update_response.assert_status_ok();
        let updated: Value = update_response.json();
        assert_eq!(updated["quotaTokens"], 2000000);
        assert!(!updated["enabled"].as_bool().unwrap());

        // Step 5: Regenerate API key
        let regen_response = server
            .post(&format!("/api/users/{}/regenerate-key", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        regen_response.assert_status_ok();
        let regenerated: Value = regen_response.json();
        let new_key = regenerated["apiKey"].as_str().unwrap();
        assert_ne!(new_key, original_key);

        // Step 6: Check user usage
        let usage_response = server
            .get(&format!("/api/usage/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        usage_response.assert_status_ok();
        let usage: Value = usage_response.json();
        assert_eq!(usage["userId"], user_id);
        assert_eq!(usage["totalRequests"], 0);

        // Step 7: Re-enable user
        let enable_response = server
            .put(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .json(&json!({"enabled": true}))
            .await;

        enable_response.assert_status_ok();
        let enabled: Value = enable_response.json();
        assert!(enabled["enabled"].as_bool().unwrap());

        // Step 8: Delete user
        let delete_response = server
            .delete(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        delete_response.assert_status_ok();

        // Step 9: Verify user no longer exists
        let verify_response = server
            .get(&format!("/api/users/{}", user_id))
            .add_header(cookie_header(), HeaderValue::from_str(&session).unwrap())
            .await;

        verify_response.assert_status_not_found();

        cleanup_env();
    }
}

// =============================================================================
// Scenario 7: Health Check
// =============================================================================

mod health_check_tests {
    use super::*;

    #[tokio::test]
    #[serial]
    async fn test_health_check_returns_status() {
        cleanup_env();
        let server = create_test_server().await;

        let response = server.get("/healthz").await;

        response.assert_status_ok();

        let body: Value = response.json();
        assert!(body["status"].is_string());
        assert!(body["database_connected"].as_bool().unwrap());

        cleanup_env();
    }
}
