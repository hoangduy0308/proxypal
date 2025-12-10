use std::sync::Arc;

pub mod cliproxy;
pub mod crypto;
pub mod db;
pub mod middleware;
pub mod routes;

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
