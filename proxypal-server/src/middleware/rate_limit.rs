use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Extension, Json,
};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use super::api_key_auth::UserContext;

#[derive(Debug, Serialize)]
struct RateLimitError {
    success: bool,
    error: String,
    code: String,
}

impl RateLimitError {
    fn rate_limited() -> Self {
        Self {
            success: false,
            error: "Rate limit exceeded".to_string(),
            code: "RATE_LIMITED".to_string(),
        }
    }
}

pub struct RateLimiter {
    state: Arc<Mutex<HashMap<i64, (Instant, u64)>>>,
    limit: u64,
    window: Duration,
}

impl RateLimiter {
    pub fn new(requests_per_minute: u64) -> Self {
        Self {
            state: Arc::new(Mutex::new(HashMap::new())),
            limit: requests_per_minute,
            window: Duration::from_secs(60),
        }
    }

    pub async fn check(&self, user_id: i64) -> (bool, u64, u64) {
        let mut state = self.state.lock().await;
        let now = Instant::now();

        let (window_start, count) = state.get(&user_id).copied().unwrap_or((now, 0));

        let elapsed = now.duration_since(window_start);
        if elapsed >= self.window {
            state.insert(user_id, (now, 1));
            let reset_at = now.elapsed().as_secs() + self.window.as_secs();
            (true, self.limit - 1, reset_at)
        } else {
            let new_count = count + 1;
            let remaining_secs = (self.window - elapsed).as_secs();
            if new_count > self.limit {
                (false, 0, remaining_secs)
            } else {
                state.insert(user_id, (window_start, new_count));
                (true, self.limit - new_count, remaining_secs)
            }
        }
    }

    #[cfg(test)]
    pub async fn reset(&self) {
        let mut state = self.state.lock().await;
        state.clear();
    }

    #[cfg(test)]
    pub fn window_duration(&self) -> Duration {
        self.window
    }
}

pub async fn rate_limit_middleware(
    State(limiter): State<Arc<RateLimiter>>,
    Extension(user): Extension<UserContext>,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    let (allowed, remaining, reset_secs) = limiter.check(user.id).await;

    if !allowed {
        let mut response = (StatusCode::TOO_MANY_REQUESTS, Json(RateLimitError::rate_limited()))
            .into_response();

        let headers = response.headers_mut();
        headers.insert("X-RateLimit-Limit", limiter.limit.into());
        headers.insert("X-RateLimit-Remaining", 0u64.into());
        headers.insert("X-RateLimit-Reset", reset_secs.into());

        return Err(response);
    }

    let mut response = next.run(req).await;

    let headers = response.headers_mut();
    headers.insert("X-RateLimit-Limit", limiter.limit.into());
    headers.insert("X-RateLimit-Remaining", remaining.into());
    headers.insert("X-RateLimit-Reset", reset_secs.into());

    Ok(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
        middleware,
        routing::get,
        Router,
    };
    use tower::ServiceExt;

    async fn dummy_handler() -> impl IntoResponse {
        "ok"
    }

    fn create_test_app(limiter: Arc<RateLimiter>, user: UserContext) -> Router {
        Router::new()
            .route("/test", get(dummy_handler))
            .layer(middleware::from_fn_with_state(
                limiter.clone(),
                rate_limit_middleware,
            ))
            .layer(Extension(user))
            .with_state(limiter)
    }

    fn test_user(id: i64) -> UserContext {
        UserContext {
            id,
            name: format!("user{}", id),
            quota_tokens: None,
            used_tokens: 0,
            enabled: true,
        }
    }

    #[tokio::test]
    async fn test_first_60_requests_succeed() {
        let limiter = Arc::new(RateLimiter::new(60));
        let user = test_user(1);
        let app = create_test_app(limiter.clone(), user);

        for i in 0..60 {
            let response = app
                .clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();

            assert_eq!(
                response.status(),
                StatusCode::OK,
                "Request {} should succeed",
                i + 1
            );

            let remaining: u64 = response
                .headers()
                .get("X-RateLimit-Remaining")
                .unwrap()
                .to_str()
                .unwrap()
                .parse()
                .unwrap();
            assert_eq!(remaining, 59 - i as u64);
        }
    }

    #[tokio::test]
    async fn test_61st_request_returns_429() {
        let limiter = Arc::new(RateLimiter::new(60));
        let user = test_user(1);
        let app = create_test_app(limiter.clone(), user);

        for _ in 0..60 {
            let _ = app
                .clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();
        }

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "RATE_LIMITED");
    }

    #[tokio::test]
    async fn test_different_users_have_separate_limits() {
        let limiter = Arc::new(RateLimiter::new(5));
        let user1 = test_user(1);
        let user2 = test_user(2);

        let app1 = create_test_app(limiter.clone(), user1);
        let app2 = create_test_app(limiter.clone(), user2);

        for _ in 0..5 {
            let response = app1
                .clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }

        let response = app1
            .clone()
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);

        for _ in 0..5 {
            let response = app2
                .clone()
                .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
        }
    }

    #[tokio::test]
    async fn test_rate_limit_headers_are_set() {
        let limiter = Arc::new(RateLimiter::new(10));
        let user = test_user(1);
        let app = create_test_app(limiter.clone(), user);

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert!(response.headers().contains_key("X-RateLimit-Limit"));
        assert!(response.headers().contains_key("X-RateLimit-Remaining"));
        assert!(response.headers().contains_key("X-RateLimit-Reset"));

        let limit: u64 = response
            .headers()
            .get("X-RateLimit-Limit")
            .unwrap()
            .to_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(limit, 10);
    }

    #[tokio::test]
    async fn test_after_window_expires_requests_succeed() {
        use std::time::Duration;

        let limiter = Arc::new(RateLimiter::new(2));

        {
            let mut state = limiter.state.lock().await;
            let old_window_start = Instant::now() - Duration::from_secs(61);
            state.insert(1, (old_window_start, 2));
        }

        let user = test_user(1);
        let app = create_test_app(limiter.clone(), user);

        let response = app
            .oneshot(Request::builder().uri("/test").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
