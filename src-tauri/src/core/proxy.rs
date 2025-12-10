use crate::core::types::LogEntry;

/// Detect provider from model name
pub fn detect_provider_from_model(model: &str) -> String {
    let model_lower = model.to_lowercase();
    
    if model_lower.contains("claude") || model_lower.contains("sonnet") || 
       model_lower.contains("opus") || model_lower.contains("haiku") {
        return "claude".to_string();
    }
    if model_lower.contains("gpt") || model_lower.contains("codex") || 
       model_lower.starts_with("o3") || model_lower.starts_with("o1") {
        return "openai".to_string();
    }
    if model_lower.contains("gemini") {
        return "gemini".to_string();
    }
    if model_lower.contains("qwen") {
        return "qwen".to_string();
    }
    if model_lower.contains("deepseek") {
        return "deepseek".to_string();
    }
    if model_lower.contains("glm") {
        return "zhipu".to_string();
    }
    if model_lower.contains("antigravity") {
        return "antigravity".to_string();
    }
    
    "unknown".to_string()
}

/// Parse a log line into a LogEntry struct
/// Expected formats from CLIProxyAPI:
/// - "[2025-12-02 22:12:52] [info] [gin_logger.go:58] message"
/// - "[2025-12-02 22:12:52] [info] message"  
/// - "2024-01-15T10:30:45.123Z [INFO] message"
pub fn parse_log_line(line: &str) -> LogEntry {
    let line = line.trim();
    
    // Format: [timestamp] [level] [source] message
    // or: [timestamp] [level] message
    if line.starts_with('[') {
        let mut parts = Vec::new();
        let mut current_start = 0;
        let mut in_bracket = false;
        
        for (i, c) in line.char_indices() {
            if c == '[' && !in_bracket {
                in_bracket = true;
                current_start = i + 1;
            } else if c == ']' && in_bracket {
                in_bracket = false;
                parts.push(&line[current_start..i]);
                current_start = i + 1;
            }
        }
        
        // Get the message (everything after the last bracket)
        let message_start = line.rfind(']').map(|i| i + 1).unwrap_or(0);
        let message = line[message_start..].trim();
        
        if parts.len() >= 2 {
            let timestamp = parts[0].to_string();
            let level = parts[1].to_uppercase();
            
            return LogEntry {
                timestamp,
                level: normalize_log_level(&level),
                message: message.to_string(),
            };
        }
    }
    
    // Try ISO timestamp format: "2024-01-15T10:30:45.123Z [INFO] message"
    if line.len() > 20 && (line.chars().nth(4) == Some('-') || line.chars().nth(10) == Some('T')) {
        if let Some(bracket_start) = line.find('[') {
            if let Some(bracket_end) = line[bracket_start..].find(']') {
                let timestamp = line[..bracket_start].trim().to_string();
                let level = line[bracket_start + 1..bracket_start + bracket_end].to_string();
                let message = line[bracket_start + bracket_end + 1..].trim().to_string();
                
                return LogEntry {
                    timestamp,
                    level: normalize_log_level(&level),
                    message,
                };
            }
        }
    }
    
    // Try "LEVEL: message" format
    for level in &["ERROR", "WARN", "INFO", "DEBUG", "TRACE"] {
        if line.to_uppercase().starts_with(level) {
            let rest = &line[level.len()..];
            if rest.starts_with(':') || rest.starts_with(' ') {
                return LogEntry {
                    timestamp: String::new(),
                    level: level.to_string(),
                    message: rest.trim_start_matches(|c| c == ':' || c == ' ').to_string(),
                };
            }
        }
    }
    
    // Default: plain text as INFO
    LogEntry {
        timestamp: String::new(),
        level: "INFO".to_string(),
        message: line.to_string(),
    }
}

/// Normalize log level to standard format
pub fn normalize_log_level(level: &str) -> String {
    match level.to_uppercase().as_str() {
        "ERROR" | "ERR" | "E" => "ERROR".to_string(),
        "WARN" | "WARNING" | "W" => "WARN".to_string(),
        "INFO" | "I" => "INFO".to_string(),
        "DEBUG" | "DBG" | "D" => "DEBUG".to_string(),
        "TRACE" | "T" => "TRACE".to_string(),
        _ => level.to_uppercase(),
    }
}
