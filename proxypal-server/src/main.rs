use axum::{
    response::{IntoResponse, Json},
    routing::get,
    Router,
};
use serde::Serialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::services::ServeDir;
use tracing::info;

mod cliproxy;
mod crypto;
mod db;
mod middleware;
mod routes;

use cliproxy::{ProxyManagementClient, ProxyProcessManager};
use db::Database;
use middleware::rate_limit::RateLimiter;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub rate_limiter: Arc<RateLimiter>,
    pub proxy_client: Arc<dyn ProxyManagementClient>,
    pub proxy_manager: Arc<dyn ProxyProcessManager>,
}

#[derive(Serialize)]
struct HealthResponse {
    status: String,
    version: String,
    proxy_running: bool,
    proxy_pid: Option<u32>,
    uptime_seconds: Option<u64>,
    database_connected: bool,
}

async fn health_check(
    axum::extract::State(state): axum::extract::State<AppState>,
) -> impl IntoResponse {
    let proxy_running = state.proxy_manager.is_running();
    let proxy_pid = state.proxy_manager.pid();
    let uptime_seconds = state.proxy_manager.uptime_seconds();
    let database_connected = state.db.conn().is_ok();

    let status = if !database_connected {
        "error"
    } else if !proxy_running {
        "degraded"
    } else {
        "ok"
    };

    Json(HealthResponse {
        status: status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        proxy_running,
        proxy_pid,
        uptime_seconds,
        database_connected,
    })
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("proxypal_server=info".parse()?)
                .add_directive("tower_http=debug".parse()?),
        )
        .init();

    info!("Starting ProxyPal Server...");

    // Initialize database
    let db = db::init()?;
    info!("Database initialized");

    // Bootstrap admin password if not set
    bootstrap_admin_password(&db)?;

    // Get rate limit from settings or use default
    let rate_limit_rpm: u64 = db
        .get_setting("rate_limit_rpm")
        .ok()
        .flatten()
        .and_then(|s| s.parse().ok())
        .unwrap_or(60);
    
    let rate_limiter = Arc::new(RateLimiter::new(rate_limit_rpm));
    info!("Rate limiter configured: {} requests per minute", rate_limit_rpm);

    let proxy_client: Arc<dyn ProxyManagementClient> = Arc::new(cliproxy::MockProxyManagementClient::default());
    let proxy_manager: Arc<dyn ProxyProcessManager> = Arc::new(cliproxy::LocalProxyProcessManager::from_env());

    let app_state = AppState { db, rate_limiter, proxy_client, proxy_manager };

    // Build admin API routes (require session auth)
    let admin_api = Router::new()
        .nest("/auth", routes::auth::router())
        .nest("/users", routes::users::router())
        .nest("/usage", routes::usage::router())
        .nest("/providers", routes::providers::router())
        .nest("/proxy", routes::proxy::router())
        .nest("/config", routes::config::router())
        .nest("/logs", routes::logs::router());

    // Build v1 proxy routes with API key auth (no rate limiting middleware here - 
    // rate limiting is handled by checking user quota in ApiKeyAuth extractor)
    let v1_proxy_routes = routes::v1_proxy::router();

    // Build router
    let app = Router::new()
        .route("/healthz", get(health_check))
        .route("/api/health", get(health_check))
        .nest("/api", admin_api)
        .nest("/oauth", routes::providers::oauth_callback_router())
        .nest("/v1", v1_proxy_routes)
        .fallback_service(ServeDir::new("dist").append_index_html_on_directories(true))
        .with_state(app_state);

    // Get port from environment
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(3000);

    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    info!("Listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

fn bootstrap_admin_password(db: &Database) -> anyhow::Result<()> {
    use argon2::{password_hash::SaltString, Argon2, PasswordHasher};
    use rand::rngs::OsRng;

    // Check if admin password is already set
    if db.get_setting("admin_password_hash")?.is_some() {
        info!("Admin password already configured");
        return Ok(());
    }

    // First run: hash password from env var
    let password = std::env::var("ADMIN_PASSWORD")
        .map_err(|_| anyhow::anyhow!("ADMIN_PASSWORD env var required on first run"))?;

    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| anyhow::anyhow!("Failed to hash password: {}", e))?
        .to_string();

    db.set_setting("admin_password_hash", &hash)?;
    info!("Admin password initialized from ADMIN_PASSWORD env var");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};
    use cliproxy::{MockProxyManagementClient, MockProxyProcessManager};
    use tower::ServiceExt;

    fn create_test_app(proxy_running: bool) -> (Router, Arc<MockProxyProcessManager>) {
        let db = db::Database::new_in_memory().unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60));
        let proxy_client: Arc<dyn ProxyManagementClient> =
            Arc::new(MockProxyManagementClient::default());
        let mock_manager = Arc::new(MockProxyProcessManager::default());
        if proxy_running {
            mock_manager.set_running(true, 12345);
        }
        let proxy_manager: Arc<dyn ProxyProcessManager> = mock_manager.clone();

        let state = AppState {
            db,
            rate_limiter,
            proxy_client,
            proxy_manager,
        };

        let app = Router::new()
            .route("/healthz", get(health_check))
            .with_state(state);

        (app, mock_manager)
    }

    #[tokio::test]
    async fn test_health_check_proxy_running() {
        let (app, _) = create_test_app(true);

        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(health["status"], "ok");
        assert_eq!(health["proxy_running"], true);
        assert_eq!(health["database_connected"], true);
        assert!(health["proxy_pid"].as_u64().is_some());
    }

    #[tokio::test]
    async fn test_health_check_proxy_not_running() {
        let (app, _) = create_test_app(false);

        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), 200);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(health["status"], "degraded");
        assert_eq!(health["proxy_running"], false);
        assert_eq!(health["database_connected"], true);
    }

    #[tokio::test]
    async fn test_health_check_includes_version() {
        let (app, _) = create_test_app(true);

        let response = app
            .oneshot(Request::builder().uri("/healthz").body(Body::empty()).unwrap())
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(health["version"], env!("CARGO_PKG_VERSION"));
    }
}
