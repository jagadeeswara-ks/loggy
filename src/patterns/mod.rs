//! Pattern detection module

use regex::Regex;
use lazy_static::lazy_static;
use crate::db::{LogLevel, LogSource};

lazy_static! {
    // Log level patterns
    static ref ERROR_PATTERN: Regex = Regex::new(r"(?i)\b(error|err|fatal|critical|exception)\b").unwrap();
    static ref WARN_PATTERN: Regex = Regex::new(r"(?i)\b(warn|warning)\b").unwrap();
    static ref DEBUG_PATTERN: Regex = Regex::new(r"(?i)\b(debug|trace|verbose)\b").unwrap();
    
    // Stack trace patterns
    static ref STACK_TRACE: Regex = Regex::new(r"^\s+at\s+[\w.]+\(.*\)$").unwrap();
    static ref STACK_TRACE_START: Regex = Regex::new(r"(?i)(exception|traceback|stack\s*trace)").unwrap();
    
    // HTTP patterns
    static ref HTTP_ERROR: Regex = Regex::new(r"HTTP/[\d.]+\s+([45]\d{2})").unwrap();
    
    // Database patterns
    static ref SQL_ERROR: Regex = Regex::new(r"(?i)(sql\s*error|database\s*error|deadlock|timeout)").unwrap();
    
    // JSON parse errors
    static ref JSON_ERROR: Regex = Regex::new(r"(?i)(json\.parse|invalid\s*json|unexpected\s*token)").unwrap();
}

#[allow(dead_code)]
pub struct PatternDetector {
    detected_patterns: std::sync::Mutex<Vec<DetectedPattern>>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct DetectedPattern {
    pub id: Option<u64>,
    pub pattern: String,
    pub pattern_type: PatternType,
    pub count: u64,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub enum PatternType {
    StackTrace,
    HttpError,
    SqlError,
    JsonError,
    Timestamp,
    Url,
    IpAddress,
    Custom(String),
}

impl PatternDetector {
    pub fn new() -> Self {
        Self {
            detected_patterns: std::sync::Mutex::new(Vec::new()),
        }
    }
    
    /// Get detected patterns for API response
    #[allow(dead_code)]
    pub fn get_detected_patterns(&self) -> Vec<DetectedPattern> {
        self.detected_patterns.lock().unwrap().clone()
    }
    
    /// Detect patterns in a log message
    pub fn detect(&self, message: &str) -> Vec<String> {
        let mut detected = Vec::new();
        
        if ERROR_PATTERN.is_match(message) {
            detected.push("error".to_string());
        }
        if WARN_PATTERN.is_match(message) {
            detected.push("warning".to_string());
        }
        if STACK_TRACE_START.is_match(message) {
            detected.push("stack_trace".to_string());
        }
        if HTTP_ERROR.is_match(message) {
            detected.push("http_error".to_string());
        }
        if SQL_ERROR.is_match(message) {
            detected.push("sql_error".to_string());
        }
        if JSON_ERROR.is_match(message) {
            detected.push("json_error".to_string());
        }
        
        detected
    }
    
    /// Record a detected pattern
    pub fn record_pattern(&self, pattern: String) {
        let mut patterns = self.detected_patterns.lock().unwrap();
        
        // Convert string to PatternType
        let pattern_type = match pattern.as_str() {
            "error" => PatternType::Custom("error".to_string()),
            "warning" => PatternType::Custom("warning".to_string()),
            "stack_trace" => PatternType::StackTrace,
            "http_error" => PatternType::HttpError,
            "sql_error" => PatternType::SqlError,
            "json_error" => PatternType::JsonError,
            _ => PatternType::Custom(pattern.clone()),
        };
        
        // Increment count for existing pattern
        if let Some(existing) = patterns.iter_mut().find(|p| p.pattern == pattern) {
            existing.count += 1;
        } else {
            // Add new pattern
            patterns.push(DetectedPattern {
                id: None,
                pattern,
                pattern_type,
                count: 1,
            });
        }
    }
    
    /// Extract log level from message content
    pub fn extract_log_level(message: &str) -> (LogLevel, String) {
        let clean_message = message.trim().to_string();
        
        if ERROR_PATTERN.is_match(&clean_message) {
            (LogLevel::Error, clean_message)
        } else if WARN_PATTERN.is_match(&clean_message) {
            (LogLevel::Warn, clean_message)
        } else if DEBUG_PATTERN.is_match(&clean_message) {
            (LogLevel::Debug, clean_message)
        } else if STACK_TRACE_START.is_match(&clean_message) {
            (LogLevel::Error, clean_message)
        } else {
            (LogLevel::Info, clean_message)
        }
    }
    
    /// Detect patterns in a log message
    pub fn detect_patterns(&self, message: &str) -> Vec<PatternType> {
        let mut patterns = Vec::new();
        
        if STACK_TRACE_START.is_match(message) || STACK_TRACE.is_match(message) {
            patterns.push(PatternType::StackTrace);
        }
        
        if HTTP_ERROR.is_match(message) {
            patterns.push(PatternType::HttpError);
        }
        
        if SQL_ERROR.is_match(message) {
            patterns.push(PatternType::SqlError);
        }
        
        if JSON_ERROR.is_match(message) {
            patterns.push(PatternType::JsonError);
        }
        
        patterns
    }
    
    /// Generate auto-filters based on detected patterns
    pub fn generate_filters(&self, messages: &[String]) -> Vec<AutoFilter> {
        let mut filters = Vec::new();
        
        // Count pattern occurrences
        let mut error_count = 0;
        let mut warning_count = 0;
        let mut stack_trace_count = 0;
        let mut http_error_count = 0;
        let mut sql_error_count = 0;
        
        for msg in messages {
            if ERROR_PATTERN.is_match(msg) || STACK_TRACE_START.is_match(msg) {
                error_count += 1;
            }
            if WARN_PATTERN.is_match(msg) {
                warning_count += 1;
            }
            if STACK_TRACE_START.is_match(msg) || STACK_TRACE.is_match(msg) {
                stack_trace_count += 1;
            }
            if HTTP_ERROR.is_match(msg) {
                http_error_count += 1;
            }
            if SQL_ERROR.is_match(msg) {
                sql_error_count += 1;
            }
        }
        
        // Create filters based on counts
        if error_count > 0 {
            filters.push(AutoFilter {
                name: "Errors".to_string(),
                query: "level=ERROR".to_string(),
                count: error_count,
                filter_type: FilterType::Level,
            });
        }
        
        if warning_count > 0 {
            filters.push(AutoFilter {
                name: "Warnings".to_string(),
                query: "level=WARN".to_string(),
                count: warning_count,
                filter_type: FilterType::Level,
            });
        }
        
        if stack_trace_count > 0 {
            filters.push(AutoFilter {
                name: "Stack Traces".to_string(),
                query: "pattern=stack_trace".to_string(),
                count: stack_trace_count,
                filter_type: FilterType::Pattern,
            });
        }
        
        if http_error_count > 0 {
            filters.push(AutoFilter {
                name: "HTTP Errors".to_string(),
                query: "pattern=http_error".to_string(),
                count: http_error_count,
                filter_type: FilterType::Pattern,
            });
        }
        
        if sql_error_count > 0 {
            filters.push(AutoFilter {
                name: "Database Errors".to_string(),
                query: "pattern=sql_error".to_string(),
                count: sql_error_count,
                filter_type: FilterType::Pattern,
            });
        }
        
        filters.sort_by(|a, b| b.count.cmp(&a.count));
        filters
    }
}

#[derive(Debug, Clone)]
pub struct AutoFilter {
    pub name: String,
    pub query: String,
    pub count: u64,
    pub filter_type: FilterType,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterType {
    Level,
    Pattern,
    Custom,
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

// ============== TESTS ==============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_error_level() {
        let (level, _) = PatternDetector::extract_log_level("ERROR: something failed");
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_extract_warn_level() {
        let (level, _) = PatternDetector::extract_log_level("WARNING: deprecated method");
        assert_eq!(level, LogLevel::Warn);
    }

    #[test]
    fn test_extract_debug_level() {
        let (level, _) = PatternDetector::extract_log_level("DEBUG: trace information");
        assert_eq!(level, LogLevel::Debug);
    }

    #[test]
    fn test_extract_info_level() {
        let (level, _) = PatternDetector::extract_log_level("Server started on port 8080");
        assert_eq!(level, LogLevel::Info);
    }

    #[test]
    fn test_extract_exception_level() {
        let (level, _) = PatternDetector::extract_log_level("Exception in thread main");
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_detect_stack_trace() {
        let detector = PatternDetector::new();
        let patterns = detector.detect_patterns("Exception in thread main\n\tat com.example.Main.main(Main.java:10)");
        assert!(patterns.contains(&PatternType::StackTrace));
    }

    #[test]
    fn test_detect_http_error() {
        let detector = PatternDetector::new();
        let patterns = detector.detect_patterns("HTTP/1.1 500 Internal Server Error");
        assert!(patterns.contains(&PatternType::HttpError));
    }

    #[test]
    fn test_detect_sql_error() {
        let detector = PatternDetector::new();
        let patterns = detector.detect_patterns("SQL Error: deadlock detected");
        assert!(patterns.contains(&PatternType::SqlError));
    }

    #[test]
    fn test_detect_json_error() {
        let detector = PatternDetector::new();
        let patterns = detector.detect_patterns("JSON.parse: unexpected token");
        assert!(patterns.contains(&PatternType::JsonError));
    }

    #[test]
    fn test_generate_filters() {
        let detector = PatternDetector::new();
        let messages = vec![
            "ERROR: connection failed".to_string(),
            "WARNING: deprecated".to_string(),
            "ERROR: timeout".to_string(),
            "INFO: server started".to_string(),
        ];
        
        let filters = detector.generate_filters(&messages);
        
        assert!(filters.len() >= 2);
        
        let error_filter = filters.iter().find(|f| f.name == "Errors").unwrap();
        assert_eq!(error_filter.count, 2);
        
        let warning_filter = filters.iter().find(|f| f.name == "Warnings").unwrap();
        assert_eq!(warning_filter.count, 1);
    }

    #[test]
    fn test_generate_filters_empty() {
        let detector = PatternDetector::new();
        let messages: Vec<String> = vec![];
        
        let filters = detector.generate_filters(&messages);
        assert!(filters.is_empty());
    }

    #[test]
    fn test_auto_filter_sorting() {
        let detector = PatternDetector::new();
        let messages = vec![
            "ERROR: first".to_string(),
            "ERROR: second".to_string(),
            "ERROR: third".to_string(),
            "WARNING: warning".to_string(),
        ];
        
        let filters = detector.generate_filters(&messages);
        
        // Errors should be first (highest count)
        assert_eq!(filters[0].name, "Errors");
        assert_eq!(filters[0].count, 3);
    }
}
