use axum::{
    async_trait,
    extract::FromRequestParts,
    http::{request::Parts, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use axum::extract::FromRef;
use axum_extra::extract::cookie::CookieJar;
use serde::Serialize;

use crate::{db::sessions::Session, AppState};

pub struct AdminSession {
    pub session: Session,
}

#[derive(Debug, Serialize)]
pub struct AuthError {
    pub success: bool,
    pub error: String,
    pub code: String,
}

impl AuthError {
    fn unauthorized() -> Self {
        Self {
            success: false,
            error: "Unauthorized".to_string(),
            code: "UNAUTHORIZED".to_string(),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        (StatusCode::UNAUTHORIZED, Json(self)).into_response()
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for AdminSession
where
    AppState: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = AppState::from_ref(state);

        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthError::unauthorized().into_response())?;

        let session_id = jar
            .get("session")
            .map(|c| c.value().to_string())
            .ok_or_else(|| AuthError::unauthorized().into_response())?;

        let session = app_state
            .db
            .get_session(&session_id)
            .map_err(|_| AuthError::unauthorized().into_response())?
            .ok_or_else(|| AuthError::unauthorized().into_response())?;

        let _ = app_state.db.update_session_access(&session_id);

        Ok(AdminSession { session })
    }
}
