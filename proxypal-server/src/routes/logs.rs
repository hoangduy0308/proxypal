use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::middleware::admin_auth::AdminSession;
use crate::AppState;

#[derive(Debug, Clone, Deserialize)]
pub struct LogsQuery {
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
    pub user_id: Option<i64>,
    pub provider: Option<String>,
    pub status: Option<String>,
}

fn default_limit() -> i64 {
    100
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogEntry {
    pub id: i64,
    pub timestamp: String,
    pub user_id: i64,
    pub user_name: String,
    pub provider: String,
    pub model: String,
    pub tokens_input: i64,
    pub tokens_output: i64,
    pub duration_ms: i64,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogsResponse {
    pub logs: Vec<LogEntry>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug)]
pub enum LogsError {
    Internal(String),
}

impl IntoResponse for LogsError {
    fn into_response(self) -> axum::response::Response {
        let (status, code, message) = match self {
            Self::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", msg),
        };

        let body = serde_json::json!({
            "success": false,
            "error": message,
            "code": code
        });

        (status, Json(body)).into_response()
    }
}

pub async fn get_logs(
    _session: AdminSession,
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Result<Json<LogsResponse>, LogsError> {
    let limit = query.limit.min(1000).max(1);
    let offset = query.offset.max(0);

    let (logs, total) = state
        .db
        .get_request_logs_paginated(
            limit,
            offset,
            query.user_id,
            query.provider.as_deref(),
            query.status.as_deref(),
        )
        .map_err(|e| LogsError::Internal(e.to_string()))?;

    Ok(Json(LogsResponse {
        logs,
        total,
        limit,
        offset,
    }))
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(get_logs))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::Database;
    use tempfile::tempdir;

    fn setup_test_db() -> Database {
        let dir = tempdir().unwrap();
        let path = dir.path().join("test.db");
        let db = Database::new(path).unwrap();

        db.create_user("testuser", None).unwrap();

        db.with_conn(|conn| {
            conn.execute(
                "INSERT INTO usage_logs (user_id, provider, model, tokens_input, tokens_output, request_time_ms, status, timestamp)
                 VALUES (1, 'claude', 'claude-3-opus', 100, 200, 1500, 'success', datetime('now'))",
                [],
            )?;
            conn.execute(
                "INSERT INTO usage_logs (user_id, provider, model, tokens_input, tokens_output, request_time_ms, status, timestamp)
                 VALUES (1, 'openai', 'gpt-4', 50, 100, 800, 'success', datetime('now'))",
                [],
            )?;
            conn.execute(
                "INSERT INTO usage_logs (user_id, provider, model, tokens_input, tokens_output, request_time_ms, status, timestamp)
                 VALUES (1, 'claude', 'claude-3-opus', 30, 0, 200, 'error', datetime('now'))",
                [],
            )?;
            Ok(())
        })
        .unwrap();

        db
    }

    #[test]
    fn test_get_logs_returns_all_logs() {
        let db = setup_test_db();
        let (logs, total) = db
            .get_request_logs_paginated(100, 0, None, None, None)
            .unwrap();

        assert_eq!(total, 3);
        assert_eq!(logs.len(), 3);
        assert_eq!(logs[0].user_name, "testuser");
    }

    #[test]
    fn test_get_logs_filters_by_provider() {
        let db = setup_test_db();
        let (logs, total) = db
            .get_request_logs_paginated(100, 0, None, Some("claude"), None)
            .unwrap();

        assert_eq!(total, 2);
        assert!(logs.iter().all(|l| l.provider == "claude"));
    }

    #[test]
    fn test_get_logs_filters_by_status() {
        let db = setup_test_db();
        let (logs, total) = db
            .get_request_logs_paginated(100, 0, None, None, Some("error"))
            .unwrap();

        assert_eq!(total, 1);
        assert_eq!(logs[0].status, "error");
    }

    #[test]
    fn test_get_logs_pagination() {
        let db = setup_test_db();

        let (logs1, _) = db
            .get_request_logs_paginated(2, 0, None, None, None)
            .unwrap();
        assert_eq!(logs1.len(), 2);

        let (logs2, _) = db
            .get_request_logs_paginated(2, 2, None, None, None)
            .unwrap();
        assert_eq!(logs2.len(), 1);
    }

    #[test]
    fn test_default_limit() {
        assert_eq!(default_limit(), 100);
    }
}
