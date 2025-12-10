use axum::{
    body::Bytes,
    extract::State,
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::cliproxy::ProxyResponse;
use crate::middleware::api_key_auth::{ApiKeyAuth, UserContext};
use crate::AppState;

#[derive(Debug, Serialize)]
struct ModelsResponse {
    object: &'static str,
    data: Vec<ModelInfo>,
}

#[derive(Debug, Serialize)]
struct ModelInfo {
    id: String,
    object: &'static str,
    created: i64,
    owned_by: String,
}

#[derive(Debug, Deserialize)]
struct UsageInfo {
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    #[allow(dead_code)]
    total_tokens: Option<i64>,
}

#[derive(Debug, Deserialize)]
struct CompletionResponse {
    model: Option<String>,
    usage: Option<UsageInfo>,
}

fn extract_provider_from_model(model: &str) -> &str {
    if model.starts_with("gpt-") || model.starts_with("o1") || model.starts_with("o3") {
        "openai"
    } else if model.starts_with("claude-") {
        "anthropic"
    } else if model.starts_with("gemini-") {
        "google"
    } else {
        "unknown"
    }
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/models", get(get_models))
        .route("/chat/completions", post(chat_completions))
        .route("/completions", post(completions))
        .route("/embeddings", post(embeddings))
}

async fn get_models(ApiKeyAuth { user: _ }: ApiKeyAuth) -> impl IntoResponse {
    let models = vec![
        ModelInfo {
            id: "gpt-4o".to_string(),
            object: "model",
            created: 1700000000,
            owned_by: "openai".to_string(),
        },
        ModelInfo {
            id: "gpt-4o-mini".to_string(),
            object: "model",
            created: 1700000000,
            owned_by: "openai".to_string(),
        },
        ModelInfo {
            id: "claude-sonnet-4-20250514".to_string(),
            object: "model",
            created: 1700000000,
            owned_by: "anthropic".to_string(),
        },
        ModelInfo {
            id: "gemini-2.5-pro".to_string(),
            object: "model",
            created: 1700000000,
            owned_by: "google".to_string(),
        },
    ];

    Json(ModelsResponse {
        object: "list",
        data: models,
    })
}

async fn forward_and_log(
    state: &AppState,
    user: &UserContext,
    path: &str,
    method: Method,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Response> {
    let start = Instant::now();

    let proxy_response = state
        .proxy_client
        .forward_request(path, method, headers, body.clone())
        .await
        .map_err(|e| {
            (
                StatusCode::BAD_GATEWAY,
                Json(serde_json::json!({
                    "error": {
                        "message": format!("Proxy error: {}", e),
                        "type": "proxy_error",
                        "code": "BAD_GATEWAY"
                    }
                })),
            )
                .into_response()
        })?;

    let duration_ms = start.elapsed().as_millis() as i64;

    let (model, tokens_input, tokens_output) = parse_usage(&proxy_response.body);
    let provider = extract_provider_from_model(&model);

    let status_str = if proxy_response.status >= 200 && proxy_response.status < 300 {
        "success"
    } else {
        "error"
    };

    if let Err(e) = state.db.log_usage(
        user.id,
        provider,
        &model,
        tokens_input,
        tokens_output,
        duration_ms,
        status_str,
    ) {
        tracing::error!("Failed to log usage: {}", e);
    }

    Ok(build_response(proxy_response))
}

fn parse_usage(body: &Bytes) -> (String, i64, i64) {
    let parsed: Result<CompletionResponse, _> = serde_json::from_slice(body);
    match parsed {
        Ok(resp) => {
            let model = resp.model.unwrap_or_else(|| "unknown".to_string());
            let usage = resp.usage.unwrap_or(UsageInfo {
                prompt_tokens: None,
                completion_tokens: None,
                total_tokens: None,
            });
            (
                model,
                usage.prompt_tokens.unwrap_or(0),
                usage.completion_tokens.unwrap_or(0),
            )
        }
        Err(_) => ("unknown".to_string(), 0, 0),
    }
}

fn build_response(proxy_response: ProxyResponse) -> Response {
    let status = StatusCode::from_u16(proxy_response.status).unwrap_or(StatusCode::OK);
    let mut response = (status, proxy_response.body).into_response();

    let resp_headers = response.headers_mut();
    for (key, value) in proxy_response.headers.iter() {
        if key != http::header::TRANSFER_ENCODING && key != http::header::CONNECTION {
            resp_headers.insert(key.clone(), value.clone());
        }
    }

    response
}

async fn chat_completions(
    State(state): State<AppState>,
    ApiKeyAuth { user }: ApiKeyAuth,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Response> {
    forward_and_log(&state, &user, "/v1/chat/completions", Method::POST, headers, body).await
}

async fn completions(
    State(state): State<AppState>,
    ApiKeyAuth { user }: ApiKeyAuth,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Response> {
    forward_and_log(&state, &user, "/v1/completions", Method::POST, headers, body).await
}

async fn embeddings(
    State(state): State<AppState>,
    ApiKeyAuth { user }: ApiKeyAuth,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Response, Response> {
    forward_and_log(&state, &user, "/v1/embeddings", Method::POST, headers, body).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cliproxy::{MockProxyManagementClient, MockProxyProcessManager, ProxyResponse};
    use crate::db::Database;
    use crate::middleware::rate_limit::RateLimiter;
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::sync::Arc;
    use tower::ServiceExt;

    fn create_test_state() -> (AppState, Arc<MockProxyManagementClient>) {
        let db = Database::new_in_memory().unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60));
        let mock_client = Arc::new(MockProxyManagementClient::default());
        let proxy_manager = Arc::new(MockProxyProcessManager::default());

        let state = AppState {
            db,
            rate_limiter,
            proxy_client: mock_client.clone(),
            proxy_manager,
        };

        (state, mock_client)
    }

    fn create_test_app(state: AppState) -> Router {
        router().with_state(state)
    }

    fn mock_chat_response() -> ProxyResponse {
        let body = serde_json::json!({
            "id": "chatcmpl-123",
            "object": "chat.completion",
            "model": "gpt-4o",
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            },
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": "Hello!"
                }
            }]
        });
        ProxyResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: Bytes::from(serde_json::to_vec(&body).unwrap()),
        }
    }

    #[tokio::test]
    async fn test_missing_authorization_returns_401() {
        let (state, _) = create_test_state();
        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat/completions")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","messages":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "UNAUTHORIZED");
    }

    #[tokio::test]
    async fn test_invalid_api_key_returns_401() {
        let (state, _) = create_test_state();
        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat/completions")
                    .header("Authorization", "Bearer sk-invalid-key-12345")
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","messages":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn test_disabled_user_returns_403() {
        let (state, mock_client) = create_test_state();
        let (user, api_key) = state.db.create_user("testuser", None).unwrap();
        state.db.update_user(user.id, None, None, Some(false)).unwrap();

        *mock_client.forward_response.lock().unwrap() = Some(mock_chat_response());

        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","messages":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "FORBIDDEN");
    }

    #[tokio::test]
    async fn test_user_at_quota_returns_429() {
        let (state, mock_client) = create_test_state();
        let (user, api_key) = state.db.create_user("testuser", Some(100)).unwrap();
        state
            .db
            .with_conn(|conn| {
                conn.execute("UPDATE users SET used_tokens = 100 WHERE id = ?1", [user.id])?;
                Ok(())
            })
            .unwrap();

        *mock_client.forward_response.lock().unwrap() = Some(mock_chat_response());

        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","messages":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["code"], "QUOTA_EXCEEDED");
    }

    #[tokio::test]
    async fn test_valid_request_forwards_to_proxy_and_returns_response() {
        let (state, mock_client) = create_test_state();
        let (_, api_key) = state.db.create_user("testuser", None).unwrap();

        *mock_client.forward_response.lock().unwrap() = Some(mock_chat_response());

        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","messages":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["model"], "gpt-4o");
        assert_eq!(json["usage"]["prompt_tokens"], 100);
        assert_eq!(json["usage"]["completion_tokens"], 50);

        let calls = mock_client.call_log.lock().unwrap().clone();
        assert!(calls.iter().any(|c| c.contains("forward_request")));
    }

    #[tokio::test]
    async fn test_usage_is_logged_after_successful_request() {
        let (state, mock_client) = create_test_state();
        let (user, api_key) = state.db.create_user("testuser", None).unwrap();

        *mock_client.forward_response.lock().unwrap() = Some(mock_chat_response());

        let app = create_test_app(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/chat/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","messages":[]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let usage = state.db.get_user_usage(user.id, "all").unwrap();
        assert_eq!(usage.total_requests, 1);
        assert_eq!(usage.total_tokens_input, 100);
        assert_eq!(usage.total_tokens_output, 50);

        let updated_user = state.db.get_user_by_id(user.id).unwrap().unwrap();
        assert_eq!(updated_user.used_tokens, 150);
    }

    #[tokio::test]
    async fn test_get_models_returns_model_list() {
        let (state, _) = create_test_state();
        let (_, api_key) = state.db.create_user("testuser", None).unwrap();

        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/models")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["object"], "list");
        assert!(json["data"].as_array().unwrap().len() > 0);
        assert!(json["data"]
            .as_array()
            .unwrap()
            .iter()
            .any(|m| m["id"] == "gpt-4o"));
    }

    #[tokio::test]
    async fn test_completions_endpoint_forwards_request() {
        let (state, mock_client) = create_test_state();
        let (_, api_key) = state.db.create_user("testuser", None).unwrap();

        *mock_client.forward_response.lock().unwrap() = Some(mock_chat_response());

        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/completions")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"gpt-4o","prompt":"Hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let calls = mock_client.call_log.lock().unwrap().clone();
        assert!(calls.iter().any(|c| c.contains("/v1/completions")));
    }

    #[tokio::test]
    async fn test_embeddings_endpoint_forwards_request() {
        let (state, mock_client) = create_test_state();
        let (_, api_key) = state.db.create_user("testuser", None).unwrap();

        let embed_response = ProxyResponse {
            status: 200,
            headers: HeaderMap::new(),
            body: Bytes::from(
                serde_json::to_vec(&serde_json::json!({
                    "object": "list",
                    "data": [{"embedding": [0.1, 0.2, 0.3]}],
                    "model": "text-embedding-ada-002",
                    "usage": {"prompt_tokens": 10, "total_tokens": 10}
                }))
                .unwrap(),
            ),
        };
        *mock_client.forward_response.lock().unwrap() = Some(embed_response);

        let app = create_test_app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/embeddings")
                    .header("Authorization", format!("Bearer {}", api_key))
                    .header("Content-Type", "application/json")
                    .body(Body::from(r#"{"model":"text-embedding-ada-002","input":"Hello"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let calls = mock_client.call_log.lock().unwrap().clone();
        assert!(calls.iter().any(|c| c.contains("/v1/embeddings")));
    }

    #[test]
    fn test_extract_provider_from_model() {
        assert_eq!(extract_provider_from_model("gpt-4o"), "openai");
        assert_eq!(extract_provider_from_model("gpt-3.5-turbo"), "openai");
        assert_eq!(extract_provider_from_model("o1-preview"), "openai");
        assert_eq!(extract_provider_from_model("claude-3-opus"), "anthropic");
        assert_eq!(extract_provider_from_model("claude-sonnet-4-20250514"), "anthropic");
        assert_eq!(extract_provider_from_model("gemini-2.5-pro"), "google");
        assert_eq!(extract_provider_from_model("some-other-model"), "unknown");
    }

    #[test]
    fn test_parse_usage_with_valid_response() {
        let body = serde_json::json!({
            "model": "gpt-4o",
            "usage": {
                "prompt_tokens": 100,
                "completion_tokens": 50,
                "total_tokens": 150
            }
        });
        let bytes = Bytes::from(serde_json::to_vec(&body).unwrap());

        let (model, input, output) = parse_usage(&bytes);
        assert_eq!(model, "gpt-4o");
        assert_eq!(input, 100);
        assert_eq!(output, 50);
    }

    #[test]
    fn test_parse_usage_with_missing_usage() {
        let body = serde_json::json!({
            "model": "gpt-4o"
        });
        let bytes = Bytes::from(serde_json::to_vec(&body).unwrap());

        let (model, input, output) = parse_usage(&bytes);
        assert_eq!(model, "gpt-4o");
        assert_eq!(input, 0);
        assert_eq!(output, 0);
    }

    #[test]
    fn test_parse_usage_with_invalid_json() {
        let bytes = Bytes::from("not json");

        let (model, input, output) = parse_usage(&bytes);
        assert_eq!(model, "unknown");
        assert_eq!(input, 0);
        assert_eq!(output, 0);
    }
}
