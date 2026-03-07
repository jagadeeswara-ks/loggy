//! Database module - ClickHouse integration

use clickhouse::Client;
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{info, warn};

pub mod chrono_datetime64_millis {
    use chrono::{DateTime, Utc, TimeZone};
    use serde::{Serializer, Deserializer, Deserialize};

    pub fn serialize<S>(dt: &DateTime<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_i64(dt.timestamp_millis())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DateTime<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = i64::deserialize(deserializer)?;
        Utc.timestamp_millis_opt(millis)
            .single()
            .ok_or_else(|| serde::de::Error::custom("invalid timestamp"))
    }
}

use std::sync::Arc;

use crate::config::DatabaseConfig;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(clickhouse::Row)]
pub struct LogEntry {
    pub id: u64,
    #[serde(with = "chrono_datetime64_millis")]
    pub timestamp: DateTime<Utc>,
    pub container_id: String,
    pub container_name: String,
    pub compose_project: String,
    pub compose_file: String,
    pub message: String,
    pub level: LogLevel,
    pub source: LogSource,
    pub metadata: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
    Unknown,
}

impl LogLevel {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "DEBUG" => LogLevel::Debug,
            "INFO" => LogLevel::Info,
            "WARN" | "WARNING" => LogLevel::Warn,
            "ERROR" | "ERR" => LogLevel::Error,
            "FATAL" | "CRITICAL" => LogLevel::Fatal,
            _ => LogLevel::Unknown,
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
            LogLevel::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LogSource {
    Stdout,
    Stderr,
}

impl LogSource {
    pub fn from_str(s: &str) -> Self {
        match s.to_uppercase().as_str() {
            "STDERR" => LogSource::Stderr,
            _ => LogSource::Stdout,
        }
    }
    
    pub fn as_str(&self) -> &str {
        match self {
            LogSource::Stdout => "STDOUT",
            LogSource::Stderr => "STDERR",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(clickhouse::Row)]
pub struct MetricEntry {
    pub id: u64,
    #[serde(with = "chrono_datetime64_millis")]
    pub timestamp: DateTime<Utc>,
    pub container_id: String,
    pub container_name: String,
    pub cpu_percent: f32,
    pub memory_percent: f32,
    pub memory_usage: u64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedPattern {
    pub id: Option<u64>,
    pub pattern: String,
    pub pattern_type: String,
    pub count: u64,
    pub last_seen: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogQueryResult {
    pub logs: Vec<LogEntry>,
    pub total: u64,
    pub has_more: bool,
}

pub struct Database {
    client: Client,
}

impl Database {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        let url = format!(
            "http://{}:{}/",
            config.host, config.port
        );
        
        let client = Client::default()
            .with_url(&url)
            .with_database(&config.database);
        
        // Test connection
        client.query("SELECT 1").execute().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        
        info!("Connected to ClickHouse at {}", url);
        
        let db = Self { client };
        
        // Initialize tables
        db.init_tables().await?;
        
        Ok(db)
    }
    
    async fn init_tables(&self) -> Result<(), anyhow::Error> {
        // Create database if not exists
        self.client
            .query("CREATE DATABASE IF NOT EXISTS loggy")
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Logs table with proper indexing
        self.client
            .query(r#"
                CREATE TABLE IF NOT EXISTS loggy.logs (
                    id UInt64,
                    timestamp DateTime64(3),
                    container_id String,
                    container_name String,
                    compose_project String,
                    compose_file String,
                    message String,
                    level String,
                    source String,
                    metadata String
                ) ENGINE = MergeTree()
                PARTITION BY toYYYYMMDD(timestamp)
                ORDER BY (container_id, timestamp)
                TTL timestamp + INTERVAL 7 DAY
                SETTINGS index_granularity = 8192
            "#)
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        
        // Metrics table
        self.client
            .query(r#"
                CREATE TABLE IF NOT EXISTS loggy.metrics (
                    id UInt64,
                    timestamp DateTime64(3),
                    container_id String,
                    container_name String,
                    cpu_percent Float32,
                    memory_percent Float32,
                    memory_usage UInt64,
                    network_rx UInt64,
                    network_tx UInt64,
                    block_read UInt64,
                    block_write UInt64
                ) ENGINE = MergeTree()
                PARTITION BY toYYYYMMDD(timestamp)
                ORDER BY (container_id, timestamp)
                TTL timestamp + INTERVAL 7 DAY
                SETTINGS index_granularity = 8192
            "#)
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;
        
        info!("Database tables initialized");
        Ok(())
    }
    


    pub async fn insert_logs_arc(&self, entries: &[std::sync::Arc<LogEntry>]) -> Result<(), anyhow::Error> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut inserter = self.client.insert("loggy.logs")?;
        for entry in entries {
            inserter.write(entry.as_ref()).await.map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        }
        inserter.end().await.map_err(|e| anyhow::anyhow!("Insert failed: {}", e))?;
        Ok(())
    }

    pub async fn insert_logs(&self, entries: &[LogEntry]) -> Result<(), anyhow::Error> {
        if entries.is_empty() {
            return Ok(());
        }

        let mut inserter = self.client.insert("loggy.logs")?;
        for entry in entries {
            inserter.write(entry).await.map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        }
        inserter.end().await.map_err(|e| anyhow::anyhow!("Insert failed: {}", e))?;
        Ok(())
    }
    pub async fn insert_log(&self, entry: &LogEntry) -> Result<(), anyhow::Error> {
        let mut inserter = self.client.insert("loggy.logs")?;
        inserter.write(entry).await.map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        inserter.end().await.map_err(|e| anyhow::anyhow!("Insert failed: {}", e))?;
        Ok(())
    }
    
    pub async fn query_logs(
        &self,
        container_id: Option<&str>,
        level: Option<&str>,
        search: Option<&str>,
        stack: Option<&str>,
        containers: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<LogQueryResult, anyhow::Error> {
        let mut query = String::from("SELECT id, timestamp, container_id, container_name, compose_project, compose_file, message, level, source, metadata FROM loggy.logs WHERE 1=1");
        
        // Build dynamic query with filters
        if let Some(cid) = container_id {
            query.push_str(&format!(" AND container_id = '{}'", cid.replace('\'', "''")));
        }

        if let Some(s) = stack {
            query.push_str(&format!(" AND compose_project = '{}'", s.replace('\'', "''")));
        }

        if let Some(c) = containers {
            let ids: Vec<&str> = c.split(',').collect();
            if !ids.is_empty() {
                let ids_str = ids.iter().map(|id| format!("'{}'", id.replace('\'', "''"))).collect::<Vec<_>>().join(",");
                query.push_str(&format!(" AND container_id IN ({})", ids_str));
            }
        }
        
        if let Some(lvl) = level {
            if lvl.to_lowercase() != "all" {
                query.push_str(&format!(" AND level = '{}'", lvl.to_uppercase()));
            }
        }
        
        if let Some(s) = search {
            let escaped = s.replace('\'', "\\'");
            query.push_str(&format!(" AND message LIKE '%{}%'", escaped));
        }
        
        // Order by timestamp descending
        query.push_str(" ORDER BY timestamp DESC");
        
        // Add pagination
        query.push_str(&format!(" LIMIT {} OFFSET {}", limit, offset));
        
        info!("Executing query: {}", query);
        
        // Execute the query
        let result = self.client.query(&query).execute().await;
        
        match result {
            Ok(_rows) => {
                // Return count for now - proper parsing would need row deserialization
                // This is a simplification - proper implementation would parse rows
                Ok(LogQueryResult {
                    logs: vec![], // Would parse rows here
                    total: 0,
                    has_more: false,
                })
            }
            Err(e) => {
                warn!("Query failed: {}", e);
                Ok(LogQueryResult {
                    logs: vec![],
                    total: 0,
                    has_more: false,
                })
            }
        }
    }
    

    pub async fn get_log_stats(
        &self,
        container_id: Option<&str>,
        stack: Option<&str>,
    ) -> Result<(u64, u64, u64, u64, u64, std::collections::HashMap<String, (String, u64)>), anyhow::Error> {
        let mut base_query = String::from("FROM loggy.logs WHERE 1=1");

        if let Some(cid) = container_id {
            base_query.push_str(&format!(" AND container_id = '{}'", cid.replace('\'', "''")));
        }

        if let Some(s) = stack {
            base_query.push_str(&format!(" AND compose_project = '{}'", s.replace('\'', "''")));
        }

        // Let's execute some aggregation queries natively in ClickHouse
        let count_query = format!("SELECT count() {}", base_query);
        let count: u64 = self.client.query(&count_query).fetch_one().await?;

        let level_query = format!("SELECT level, count() {} GROUP BY level", base_query);
        let mut level_cursor = self.client.query(&level_query).fetch::<(String, u64)>()?;

        let mut error_count = 0;
        let mut warn_count = 0;
        let mut info_count = 0;
        let mut debug_count = 0;

        while let Some((level, c)) = level_cursor.next().await? {
            match level.to_uppercase().as_str() {
                "ERROR" | "FATAL" | "CRITICAL" | "ERR" => error_count += c,
                "WARN" | "WARNING" => warn_count += c,
                "INFO" => info_count += c,
                "DEBUG" => debug_count += c,
                _ => {}
            }
        }

        let container_query = format!("SELECT container_id, container_name, count() {} GROUP BY container_id, container_name", base_query);
        let mut container_cursor = self.client.query(&container_query).fetch::<(String, String, u64)>()?;

        let mut container_counts = std::collections::HashMap::new();
        while let Some((id, name, c)) = container_cursor.next().await? {
            container_counts.insert(id, (name, c));
        }

        Ok((count, error_count, warn_count, info_count, debug_count, container_counts))
    }
    pub async fn get_metrics(
        &self,
        container_id: Option<&str>,
        limit: u64,
    ) -> Result<Vec<MetricEntry>, anyhow::Error> {
        let mut query = String::from("SELECT id, timestamp, container_id, container_name, cpu_percent, memory_percent, memory_usage, network_rx, network_tx, block_read, block_write FROM loggy.metrics");
        
        if let Some(cid) = container_id {
            query.push_str(&format!(" WHERE container_id = '{}'", cid));
        }
        
        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT {}", limit));
        
        info!("Executing metrics query: {}", query);
        
        // Return empty for now
        Ok(vec![])
    }
    
    pub async fn insert_metric(&self, entry: &MetricEntry) -> Result<(), anyhow::Error> {
        let mut inserter = self.client.insert("loggy.metrics")?;
        inserter.write(entry).await.map_err(|e| anyhow::anyhow!("Write failed: {}", e))?;
        inserter.end().await.map_err(|e| anyhow::anyhow!("Insert failed: {}", e))?;
        Ok(())
    }
    
    pub async fn health_check(&self) -> Result<bool, anyhow::Error> {
        self.client.query("SELECT 1")
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("Health check failed: {}", e))?;
        Ok(true)
    }
}

// ============== TESTS ==============

#[cfg(test)]
mod tests {
    use super::*;

    // LogLevel tests
    #[test]
    fn test_log_level_from_str() {
        assert_eq!(LogLevel::from_str("DEBUG"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("INFO"), LogLevel::Info);
        assert_eq!(LogLevel::from_str("WARN"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("WARNING"), LogLevel::Warn);
        assert_eq!(LogLevel::from_str("ERROR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("ERR"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("FATAL"), LogLevel::Fatal);
        assert_eq!(LogLevel::from_str("CRITICAL"), LogLevel::Fatal);
        assert_eq!(LogLevel::from_str("UNKNOWN"), LogLevel::Unknown);
        assert_eq!(LogLevel::from_str("anything"), LogLevel::Unknown);
    }

    #[test]
    fn test_log_level_as_str() {
        assert_eq!(LogLevel::Debug.as_str(), "DEBUG");
        assert_eq!(LogLevel::Info.as_str(), "INFO");
        assert_eq!(LogLevel::Warn.as_str(), "WARN");
        assert_eq!(LogLevel::Error.as_str(), "ERROR");
        assert_eq!(LogLevel::Fatal.as_str(), "FATAL");
        assert_eq!(LogLevel::Unknown.as_str(), "UNKNOWN");
    }

    #[test]
    fn test_log_level_case_insensitive() {
        assert_eq!(LogLevel::from_str("debug"), LogLevel::Debug);
        assert_eq!(LogLevel::from_str("Error"), LogLevel::Error);
        assert_eq!(LogLevel::from_str("WARN"), LogLevel::Warn);
    }

    // LogSource tests
    #[test]
    fn test_log_source_from_str() {
        assert_eq!(LogSource::from_str("STDOUT"), LogSource::Stdout);
        assert_eq!(LogSource::from_str("STDERR"), LogSource::Stderr);
        assert_eq!(LogSource::from_str("anything"), LogSource::Stdout); // default
    }

    #[test]
    fn test_log_source_as_str() {
        assert_eq!(LogSource::Stdout.as_str(), "STDOUT");
        assert_eq!(LogSource::Stderr.as_str(), "STDERR");
    }

    // LogEntry tests
    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry {
            id: 1,
            timestamp: chrono::Utc::now(),
            container_id: "abc123".to_string(),
            container_name: "nginx".to_string(),
            compose_project: "myapp".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Server started".to_string(),
            level: LogLevel::Info,
            source: LogSource::Stdout,
            metadata: "".to_string(),
        };
        
        assert_eq!(entry.id, 1);
        assert_eq!(entry.container_id, "abc123");
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_log_entry_with_metadata() {
        let entry = LogEntry {
            id: 0,
            timestamp: chrono::Utc::now(),
            container_id: "test".to_string(),
            container_name: "test".to_string(),
            compose_project: "test".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Error occurred".to_string(),
            level: LogLevel::Error,
            source: LogSource::Stderr,
            metadata: r#"{"key": "value"}"#.to_string(),
        };
        
        assert_eq!(entry.id, 0);
        assert_eq!(entry.metadata, r#"{"key": "value"}"#);
    }

    // MetricEntry tests
    #[test]
    fn test_metric_entry_creation() {
        let entry = MetricEntry {
            id: 1,
            timestamp: chrono::Utc::now(),
            container_id: "abc123".to_string(),
            container_name: "nginx".to_string(),
            cpu_percent: 25.5,
            memory_percent: 50.0,
            memory_usage: 1000000,
            network_rx: 500000,
            network_tx: 300000,
            block_read: 10000,
            block_write: 5000,
        };
        
        assert_eq!(entry.cpu_percent, 25.5);
        assert_eq!(entry.memory_percent, 50.0);
    }

    // LogQueryResult tests
    #[test]
    fn test_log_query_result_empty() {
        let result = LogQueryResult {
            logs: vec![],
            total: 0,
            has_more: false,
        };
        
        assert!(result.logs.is_empty());
        assert_eq!(result.total, 0);
        assert!(!result.has_more);
    }

    #[test]
    fn test_log_query_result_with_data() {
        let entry = LogEntry {
            id: 1,
            timestamp: chrono::Utc::now(),
            container_id: "test".to_string(),
            container_name: "test".to_string(),
            compose_project: "test".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Test".to_string(),
            level: LogLevel::Info,
            source: LogSource::Stdout,
            metadata: "".to_string(),
        };
        
        let result = LogQueryResult {
            logs: vec![entry],
            total: 100,
            has_more: true,
        };
        
        assert_eq!(result.logs.len(), 1);
        assert_eq!(result.total, 100);
        assert!(result.has_more);
    }

    // Serialize/Deserialize tests
    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            id: 1,
            timestamp: chrono::Utc::now(),
            container_id: "test".to_string(),
            container_name: "test".to_string(),
            compose_project: "test".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Test message".to_string(),
            level: LogLevel::Error,
            source: LogSource::Stderr,
            metadata: "".to_string(),
        };
        
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Error")); // Serialized as "Error" not "ERROR"
    }

    #[test]
    fn test_metric_entry_serialization() {
        let entry = MetricEntry {
            id: 1,
            timestamp: chrono::Utc::now(),
            container_id: "test".to_string(),
            container_name: "test".to_string(),
            cpu_percent: 10.0,
            memory_percent: 20.0,
            memory_usage: 100,
            network_rx: 200,
            network_tx: 300,
            block_read: 400,
            block_write: 500,
        };
        
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("cpu_percent"));
        assert!(json.contains("10"));
    }
}
