use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::Database;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageLog {
    pub id: i64,
    pub user_id: i64,
    pub provider: String,
    pub model: String,
    pub tokens_input: i64,
    pub tokens_output: i64,
    pub request_time_ms: i64,
    pub status: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UsageStats {
    pub total_requests: i64,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUsage {
    pub provider: String,
    pub requests: i64,
    pub tokens_input: i64,
    pub tokens_output: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyUsage {
    pub date: String,
    pub requests: i64,
    pub tokens_input: i64,
    pub tokens_output: i64,
}

impl Database {
    pub fn log_usage(&self, user_id: i64, provider: &str, model: &str, tokens_input: i64, tokens_output: i64, request_time_ms: i64, status: &str) -> Result<()> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO usage_logs (user_id, provider, model, tokens_input, tokens_output, request_time_ms, status) VALUES (?, ?, ?, ?, ?, ?, ?)",
                rusqlite::params![user_id, provider, model, tokens_input, tokens_output, request_time_ms, status],
            )?;
            
            // Update user's used_tokens
            conn.execute(
                "UPDATE users SET used_tokens = used_tokens + ?, last_used_at = datetime('now') WHERE id = ?",
                rusqlite::params![tokens_input + tokens_output, user_id],
            )?;
            
            Ok(())
        })
    }

    pub fn get_usage_stats(&self, period: &str) -> Result<UsageStats> {
        self.with_conn(|conn| {
            let date_filter = Self::period_to_date_filter(period);
            let sql = format!(
                "SELECT COUNT(*) as total_requests, COALESCE(SUM(tokens_input), 0) as total_tokens_input, COALESCE(SUM(tokens_output), 0) as total_tokens_output FROM usage_logs{}",
                date_filter
            );
            let mut stmt = conn.prepare(&sql)?;
            let stats = stmt.query_row([], |row| {
                Ok(UsageStats {
                    total_requests: row.get(0)?,
                    total_tokens_input: row.get(1)?,
                    total_tokens_output: row.get(2)?,
                })
            })?;
            Ok(stats)
        })
    }

    pub fn get_user_usage(&self, user_id: i64, period: &str) -> Result<UsageStats> {
        self.with_conn(|conn| {
            let date_filter = Self::period_to_date_filter(period);
            let where_clause = if date_filter.is_empty() {
                " WHERE user_id = ?".to_string()
            } else {
                format!("{} AND user_id = ?", date_filter)
            };
            let sql = format!(
                "SELECT COUNT(*) as total_requests, COALESCE(SUM(tokens_input), 0) as total_tokens_input, COALESCE(SUM(tokens_output), 0) as total_tokens_output FROM usage_logs{}",
                where_clause
            );
            let mut stmt = conn.prepare(&sql)?;
            let stats = stmt.query_row([user_id], |row| {
                Ok(UsageStats {
                    total_requests: row.get(0)?,
                    total_tokens_input: row.get(1)?,
                    total_tokens_output: row.get(2)?,
                })
            })?;
            Ok(stats)
        })
    }

    pub fn get_usage_by_provider(&self, period: &str) -> Result<Vec<ProviderUsage>> {
        self.with_conn(|conn| {
            let date_filter = Self::period_to_date_filter(period);
            let sql = format!(
                "SELECT provider, COUNT(*) as requests, COALESCE(SUM(tokens_input), 0) as tokens_input, COALESCE(SUM(tokens_output), 0) as tokens_output FROM usage_logs{} GROUP BY provider ORDER BY requests DESC",
                date_filter
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], |row| {
                Ok(ProviderUsage {
                    provider: row.get(0)?,
                    requests: row.get(1)?,
                    tokens_input: row.get(2)?,
                    tokens_output: row.get(3)?,
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    pub fn get_daily_usage(&self, days: u32, user_id: Option<i64>, provider: Option<&str>) -> Result<Vec<DailyUsage>> {
        self.with_conn(|conn| {
            let mut conditions = vec![format!("timestamp >= datetime('now', '-{} days')", days)];
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            
            if let Some(uid) = user_id {
                conditions.push("user_id = ?".to_string());
                params.push(Box::new(uid));
            }
            if let Some(prov) = provider {
                conditions.push("provider = ?".to_string());
                params.push(Box::new(prov.to_string()));
            }
            
            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };
            
            let sql = format!(
                "SELECT date(timestamp) as date, COUNT(*) as requests, COALESCE(SUM(tokens_input), 0) as tokens_input, COALESCE(SUM(tokens_output), 0) as tokens_output FROM usage_logs{} GROUP BY date(timestamp) ORDER BY date DESC",
                where_clause
            );
            
            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), |row| {
                Ok(DailyUsage {
                    date: row.get(0)?,
                    requests: row.get(1)?,
                    tokens_input: row.get(2)?,
                    tokens_output: row.get(3)?,
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok(results)
        })
    }

    pub fn get_usage_logs_paginated(&self, limit: u32, offset: u32, user_id: Option<i64>, provider: Option<&str>) -> Result<(Vec<UsageLog>, u64)> {
        self.with_conn(|conn| {
            let mut conditions: Vec<String> = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
            
            if let Some(uid) = user_id {
                conditions.push("user_id = ?".to_string());
                params.push(Box::new(uid));
            }
            if let Some(prov) = provider {
                conditions.push("provider = ?".to_string());
                params.push(Box::new(prov.to_string()));
            }
            
            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };
            
            // Get total count
            let count_sql = format!("SELECT COUNT(*) FROM usage_logs{}", where_clause);
            let mut count_stmt = conn.prepare(&count_sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let total: u64 = count_stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;
            
            // Get paginated results
            let sql = format!(
                "SELECT id, user_id, provider, model, tokens_input, tokens_output, request_time_ms, COALESCE(status, 'success') as status, timestamp FROM usage_logs{} ORDER BY timestamp DESC LIMIT ? OFFSET ?",
                where_clause
            );
            
            let mut all_params = params;
            all_params.push(Box::new(limit));
            all_params.push(Box::new(offset));
            
            let mut stmt = conn.prepare(&sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = all_params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), |row| {
                Ok(UsageLog {
                    id: row.get(0)?,
                    user_id: row.get(1)?,
                    provider: row.get(2)?,
                    model: row.get(3)?,
                    tokens_input: row.get(4)?,
                    tokens_output: row.get(5)?,
                    request_time_ms: row.get(6)?,
                    status: row.get(7)?,
                    timestamp: row.get(8)?,
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok((results, total))
        })
    }

    pub fn get_request_logs_paginated(
        &self,
        limit: i64,
        offset: i64,
        user_id: Option<i64>,
        provider: Option<&str>,
        status: Option<&str>,
    ) -> Result<(Vec<crate::routes::logs::LogEntry>, i64)> {
        self.with_conn(|conn| {
            let mut conditions: Vec<String> = Vec::new();
            let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

            if let Some(uid) = user_id {
                conditions.push("ul.user_id = ?".to_string());
                params.push(Box::new(uid));
            }
            if let Some(p) = provider {
                conditions.push("ul.provider = ?".to_string());
                params.push(Box::new(p.to_string()));
            }
            if let Some(s) = status {
                conditions.push("ul.status = ?".to_string());
                params.push(Box::new(s.to_string()));
            }

            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!(" WHERE {}", conditions.join(" AND "))
            };

            let count_sql = format!("SELECT COUNT(*) FROM usage_logs ul{}", where_clause);
            let mut count_stmt = conn.prepare(&count_sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
            let total: i64 = count_stmt.query_row(param_refs.as_slice(), |row| row.get(0))?;

            let query_sql = format!(
                "SELECT ul.id, ul.timestamp, ul.user_id, u.name, ul.provider, ul.model, 
                        ul.tokens_input, ul.tokens_output, ul.request_time_ms, COALESCE(ul.status, 'success') as status
                 FROM usage_logs ul
                 LEFT JOIN users u ON ul.user_id = u.id
                 {}
                 ORDER BY ul.timestamp DESC
                 LIMIT ? OFFSET ?",
                where_clause
            );

            let mut all_params = params;
            all_params.push(Box::new(limit));
            all_params.push(Box::new(offset));

            let mut stmt = conn.prepare(&query_sql)?;
            let param_refs: Vec<&dyn rusqlite::ToSql> =
                all_params.iter().map(|p| p.as_ref()).collect();
            let rows = stmt.query_map(param_refs.as_slice(), |row| {
                Ok(crate::routes::logs::LogEntry {
                    id: row.get(0)?,
                    timestamp: row.get(1)?,
                    user_id: row.get(2)?,
                    user_name: row
                        .get::<_, Option<String>>(3)?
                        .unwrap_or_else(|| "Unknown".to_string()),
                    provider: row.get(4)?,
                    model: row.get(5)?,
                    tokens_input: row.get(6)?,
                    tokens_output: row.get(7)?,
                    duration_ms: row.get(8)?,
                    status: row.get(9)?,
                })
            })?;
            let mut results = Vec::new();
            for row in rows {
                results.push(row?);
            }
            Ok((results, total))
        })
    }

    fn period_to_date_filter(period: &str) -> String {
        match period {
            "today" => " WHERE timestamp >= datetime('now', 'start of day')".to_string(),
            "week" => " WHERE timestamp >= datetime('now', '-7 days')".to_string(),
            "month" => " WHERE timestamp >= datetime('now', '-30 days')".to_string(),
            "all" | _ => String::new(),
        }
    }

    pub fn get_total_requests(&self) -> Result<i64> {
        self.with_conn(|conn| {
            let count: i64 = conn.query_row(
                "SELECT COUNT(*) FROM usage_logs",
                [],
                |row| row.get(0),
            )?;
            Ok(count)
        })
    }
}
