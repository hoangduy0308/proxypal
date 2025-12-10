use axum::{
    body::Body,
    http::{Method, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;

#[derive(Debug, Serialize)]
struct CsrfError {
    success: bool,
    error: String,
    code: String,
}

impl CsrfError {
    fn forbidden() -> Self {
        Self {
            success: false,
            error: "CSRF token mismatch".to_string(),
            code: "CSRF_MISMATCH".to_string(),
        }
    }
}

pub async fn csrf_protection(
    jar: CookieJar,
    req: Request<Body>,
    next: Next,
) -> Result<Response, Response> {
    if matches!(*req.method(), Method::GET | Method::HEAD | Method::OPTIONS) {
        return Ok(next.run(req).await);
    }

    let cookie_token = jar
        .get("csrf_token")
        .map(|c| c.value().to_string());

    let header_token = req
        .headers()
        .get("X-CSRF-Token")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    match (cookie_token, header_token) {
        (Some(cookie), Some(header)) if cookie == header => Ok(next.run(req).await),
        _ => Err((StatusCode::FORBIDDEN, Json(CsrfError::forbidden())).into_response()),
    }
}
