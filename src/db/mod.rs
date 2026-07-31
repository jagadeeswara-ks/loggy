//! Database module - ClickHouse integration
//!
//! Writes go through `Inserter`, which buffers rows in RowBinary and flushes on a size
//! or time threshold. Reads use bound parameters. Neither path builds SQL by string
//! interpolation - log content is attacker-controlled (any container can emit any bytes)
//! and query filters arrive straight off HTTP query params.

use clickhouse::inserter::Inserter;
use clickhouse::{Client, Reflection};
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use tracing::{info, warn};
use std::time::Duration;
use tokio::sync::Mutex;

use crate::config::DatabaseConfig;

/// Flush thresholds. ClickHouse wants batches - single-row inserts cap throughput far
/// below what MergeTree can absorb, and each one is a separate HTTP round trip.
const MAX_BUFFERED_ROWS: u64 = 10_000;
const MAX_BUFFER_AGE: Duration = Duration::from_secs(5);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub id: Option<u64>,
    pub timestamp: DateTime<Utc>,
    pub container_id: String,
    pub container_name: String,
    pub compose_project: String,
    pub compose_file: String,
    pub message: String,
    pub level: LogLevel,
    pub source: LogSource,
    pub metadata: Option<String>,
}

/// Wire representation. Field order must match the column order in `loggy.logs`.
/// `timestamp` is i64 because DateTime64(3) is milliseconds-since-epoch in RowBinary;
/// clickhouse 0.6 has no chrono serde helpers, so the conversion is explicit.
#[derive(Debug, Reflection, Serialize, Deserialize)]
struct LogRow {
    id: u64,
    timestamp: i64,
    container_id: String,
    container_name: String,
    compose_project: String,
    compose_file: String,
    message: String,
    level: String,
    source: String,
    metadata: String,
}

impl LogRow {
    fn from_entry(e: &LogEntry) -> Self {
        Self {
            id: e.id.unwrap_or(0),
            timestamp: e.timestamp.timestamp_millis(),
            container_id: e.container_id.clone(),
            container_name: e.container_name.clone(),
            compose_project: e.compose_project.clone(),
            compose_file: e.compose_file.clone(),
            message: e.message.clone(),
            level: e.level.as_str().to_string(),
            source: e.source.as_str().to_string(),
            metadata: e.metadata.clone().unwrap_or_default(),
        }
    }

    fn into_entry(self) -> LogEntry {
        LogEntry {
            id: Some(self.id),
            timestamp: DateTime::from_timestamp_millis(self.timestamp).unwrap_or_else(Utc::now),
            container_id: self.container_id,
            container_name: self.container_name,
            compose_project: self.compose_project,
            compose_file: self.compose_file,
            message: self.message,
            level: LogLevel::from_str(&self.level),
            source: LogSource::from_str(&self.source),
            metadata: if self.metadata.is_empty() { None } else { Some(self.metadata) },
        }
    }
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
pub struct MetricEntry {
    pub id: Option<u64>,
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

#[derive(Debug, Reflection, Serialize, Deserialize)]
struct MetricRow {
    id: u64,
    timestamp: i64,
    container_id: String,
    container_name: String,
    cpu_percent: f32,
    memory_percent: f32,
    memory_usage: u64,
    network_rx: u64,
    network_tx: u64,
    block_read: u64,
    block_write: u64,
}

impl MetricRow {
    fn from_entry(e: &MetricEntry) -> Self {
        Self {
            id: e.id.unwrap_or(0),
            timestamp: e.timestamp.timestamp_millis(),
            container_id: e.container_id.clone(),
            container_name: e.container_name.clone(),
            cpu_percent: e.cpu_percent,
            memory_percent: e.memory_percent,
            memory_usage: e.memory_usage,
            network_rx: e.network_rx,
            network_tx: e.network_tx,
            block_read: e.block_read,
            block_write: e.block_write,
        }
    }

    fn into_entry(self) -> MetricEntry {
        MetricEntry {
            id: Some(self.id),
            timestamp: DateTime::from_timestamp_millis(self.timestamp).unwrap_or_else(Utc::now),
            container_id: self.container_id,
            container_name: self.container_name,
            cpu_percent: self.cpu_percent,
            memory_percent: self.memory_percent,
            memory_usage: self.memory_usage,
            network_rx: self.network_rx,
            network_tx: self.network_tx,
            block_read: self.block_read,
            block_write: self.block_write,
        }
    }
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

/// Escape LIKE metacharacters so a user-supplied search term is matched literally.
/// Binding stops injection; this stops `%` in a search box silently matching everything.
fn escape_like(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for c in s.chars() {
        if matches!(c, '\\' | '%' | '_') {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

pub struct Database {
    client: Client,
    logs: Mutex<Option<Inserter<LogRow>>>,
    metrics: Mutex<Option<Inserter<MetricRow>>>,
}

impl Database {
    pub async fn new(config: &DatabaseConfig) -> Result<Self, anyhow::Error> {
        let url = format!("http://{}:{}/", config.host, config.port);

        let client = Client::default()
            .with_url(&url)
            .with_database(&config.database);

        // Test connection
        client.query("SELECT 1").execute().await.map_err(|e| anyhow::anyhow!("{}", e))?;

        info!("Connected to ClickHouse at {}", url);

        let db = Self {
            client,
            logs: Mutex::new(None),
            metrics: Mutex::new(None),
        };

        db.init_tables().await?;
        Ok(db)
    }

    async fn init_tables(&self) -> Result<(), anyhow::Error> {
        self.client
            .query("CREATE DATABASE IF NOT EXISTS loggy")
            .execute()
            .await
            .map_err(|e| anyhow::anyhow!("{}", e))?;

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

    /// Buffer a log row. Flushes automatically once MAX_BUFFERED_ROWS or MAX_BUFFER_AGE
    /// is exceeded — `commit()` is cheap when neither threshold is hit.
    pub async fn insert_log(&self, entry: &LogEntry) -> Result<(), anyhow::Error> {
        let mut guard = self.logs.lock().await;
        if guard.is_none() {
            *guard = Some(
                self.client
                    .inserter::<LogRow>("loggy.logs")
                    .map_err(|e| anyhow::anyhow!("inserter: {}", e))?
                    .with_max_entries(MAX_BUFFERED_ROWS)
                    .with_max_duration(MAX_BUFFER_AGE),
            );
        }
        let inserter = guard.as_mut().expect("inserter initialized above");
        inserter
            .write(&LogRow::from_entry(entry))
            .await
            .map_err(|e| anyhow::anyhow!("buffer log: {}", e))?;
        inserter
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!("flush logs: {}", e))?;
        Ok(())
    }

    pub async fn insert_metric(&self, entry: &MetricEntry) -> Result<(), anyhow::Error> {
        let mut guard = self.metrics.lock().await;
        if guard.is_none() {
            *guard = Some(
                self.client
                    .inserter::<MetricRow>("loggy.metrics")
                    .map_err(|e| anyhow::anyhow!("inserter: {}", e))?
                    .with_max_entries(MAX_BUFFERED_ROWS)
                    .with_max_duration(MAX_BUFFER_AGE),
            );
        }
        let inserter = guard.as_mut().expect("inserter initialized above");
        inserter
            .write(&MetricRow::from_entry(entry))
            .await
            .map_err(|e| anyhow::anyhow!("buffer metric: {}", e))?;
        inserter
            .commit()
            .await
            .map_err(|e| anyhow::anyhow!("flush metrics: {}", e))?;
        Ok(())
    }

    /// Force-flush both buffers. Call on shutdown, or buffered rows are lost.
    pub async fn flush(&self) -> Result<(), anyhow::Error> {
        if let Some(i) = self.logs.lock().await.take() {
            i.end().await.map_err(|e| anyhow::anyhow!("flush logs: {}", e))?;
        }
        if let Some(i) = self.metrics.lock().await.take() {
            i.end().await.map_err(|e| anyhow::anyhow!("flush metrics: {}", e))?;
        }
        info!("Flushed pending rows");
        Ok(())
    }

    pub async fn query_logs(
        &self,
        container_id: Option<&str>,
        level: Option<&str>,
        search: Option<&str>,
        limit: u64,
        offset: u64,
    ) -> Result<LogQueryResult, anyhow::Error> {
        // Filters are appended as `?` placeholders and bound below — never interpolated.
        let level = level.filter(|l| l.to_lowercase() != "all");
        let mut where_sql = String::from(" WHERE 1=1");
        if container_id.is_some() {
            where_sql.push_str(" AND container_id = ?");
        }
        if level.is_some() {
            where_sql.push_str(" AND level = ?");
        }
        if search.is_some() {
            where_sql.push_str(" AND message LIKE ?");
        }

        let bind_filters = |mut q: clickhouse::query::Query| {
            if let Some(c) = container_id {
                q = q.bind(c);
            }
            if let Some(l) = level {
                q = q.bind(l.to_uppercase());
            }
            if let Some(s) = search {
                q = q.bind(format!("%{}%", escape_like(s)));
            }
            q
        };

        let total: u64 = bind_filters(
            self.client
                .query(&format!("SELECT count() FROM loggy.logs{}", where_sql)),
        )
        .fetch_one()
        .await
        .unwrap_or(0);

        let sql = format!(
            "SELECT ?fields FROM loggy.logs{} ORDER BY timestamp DESC LIMIT ? OFFSET ?",
            where_sql
        );
        let rows: Vec<LogRow> = bind_filters(self.client.query(&sql))
            .bind(limit)
            .bind(offset)
            .fetch_all()
            .await
            .map_err(|e| {
                warn!("Log query failed: {}", e);
                anyhow::anyhow!("Query failed: {}", e)
            })?;

        let logs: Vec<LogEntry> = rows.into_iter().map(LogRow::into_entry).collect();
        Ok(LogQueryResult {
            has_more: offset + (logs.len() as u64) < total,
            total,
            logs,
        })
    }

    pub async fn get_metrics(
        &self,
        container_id: Option<&str>,
        limit: u64,
    ) -> Result<Vec<MetricEntry>, anyhow::Error> {
        let mut sql = String::from("SELECT ?fields FROM loggy.metrics");
        if container_id.is_some() {
            sql.push_str(" WHERE container_id = ?");
        }
        sql.push_str(" ORDER BY timestamp DESC LIMIT ?");

        let mut q = self.client.query(&sql);
        if let Some(cid) = container_id {
            q = q.bind(cid);
        }

        let rows: Vec<MetricRow> = q.bind(limit).fetch_all().await.map_err(|e| {
            warn!("Metrics query failed: {}", e);
            anyhow::anyhow!("Query failed: {}", e)
        })?;

        Ok(rows.into_iter().map(MetricRow::into_entry).collect())
    }

    pub async fn health_check(&self) -> Result<bool, anyhow::Error> {
        self.client
            .query("SELECT 1")
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

    // ---- injection / escaping regression ----
    //
    // These are the tests that would have caught the original bug: log content and
    // query filters went into SQL by string interpolation, escaping only single quotes
    // in one of six fields.

    #[test]
    fn test_escape_like_neutralises_wildcards() {
        assert_eq!(escape_like("100%"), "100\\%");
        assert_eq!(escape_like("a_b"), "a\\_b");
        assert_eq!(escape_like("back\\slash"), "back\\\\slash");
        assert_eq!(escape_like("plain"), "plain");
    }

    #[test]
    fn test_escape_like_handles_injection_shaped_input() {
        // A container emitting this used to break the INSERT and could append SQL.
        let hostile = "'; DROP TABLE loggy.logs; --";
        let escaped = escape_like(hostile);
        // quotes are left to the bind layer; only LIKE metachars are our job here
        assert!(!escaped.contains("\\'"));
        assert_eq!(escape_like("%'; --"), "\\%'; --");
    }

    #[test]
    fn test_log_row_roundtrip_preserves_hostile_content() {
        let nasty = "line with ' quote, \\ backslash, \0 null and 100% wildcards";
        let entry = LogEntry {
            id: Some(7),
            timestamp: Utc::now(),
            container_id: "abc".into(),
            container_name: "n'ginx".into(),
            compose_project: "proj".into(),
            compose_file: "docker-compose.yml".into(),
            message: nasty.into(),
            level: LogLevel::Error,
            source: LogSource::Stderr,
            metadata: None,
        };
        let back = LogRow::from_entry(&entry).into_entry();
        assert_eq!(back.message, nasty, "message must survive verbatim");
        assert_eq!(back.container_name, "n'ginx");
        assert_eq!(back.level, LogLevel::Error);
        assert_eq!(back.source, LogSource::Stderr);
    }

    #[test]
    fn test_log_row_timestamp_millis_roundtrip() {
        let ts = DateTime::from_timestamp_millis(1_767_225_600_123).unwrap();
        let entry = LogEntry {
            id: None,
            timestamp: ts,
            container_id: "c".into(),
            container_name: "c".into(),
            compose_project: "p".into(),
            compose_file: "f".into(),
            message: "m".into(),
            level: LogLevel::Info,
            source: LogSource::Stdout,
            metadata: Some("{}".into()),
        };
        let row = LogRow::from_entry(&entry);
        assert_eq!(row.timestamp, 1_767_225_600_123);
        assert_eq!(row.into_entry().timestamp, ts);
    }

    #[test]
    fn test_empty_metadata_becomes_none() {
        let row = LogRow {
            id: 0,
            timestamp: 0,
            container_id: String::new(),
            container_name: String::new(),
            compose_project: String::new(),
            compose_file: String::new(),
            message: String::new(),
            level: "INFO".into(),
            source: "STDOUT".into(),
            metadata: String::new(),
        };
        assert!(row.into_entry().metadata.is_none());
    }

    #[test]
    fn test_metric_row_roundtrip() {
        let entry = MetricEntry {
            id: Some(3),
            timestamp: DateTime::from_timestamp_millis(1_700_000_000_000).unwrap(),
            container_id: "abc".into(),
            container_name: "nginx".into(),
            cpu_percent: 25.5,
            memory_percent: 50.0,
            memory_usage: 1_000_000,
            network_rx: 500_000,
            network_tx: 300_000,
            block_read: 10_000,
            block_write: 5_000,
        };
        let back = MetricRow::from_entry(&entry).into_entry();
        assert_eq!(back.cpu_percent, 25.5);
        assert_eq!(back.memory_usage, 1_000_000);
        assert_eq!(back.timestamp, entry.timestamp);
    }

    // ---- existing behaviour ----

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

    #[test]
    fn test_log_source_from_str() {
        assert_eq!(LogSource::from_str("STDOUT"), LogSource::Stdout);
        assert_eq!(LogSource::from_str("STDERR"), LogSource::Stderr);
        assert_eq!(LogSource::from_str("anything"), LogSource::Stdout);
    }

    #[test]
    fn test_log_source_as_str() {
        assert_eq!(LogSource::Stdout.as_str(), "STDOUT");
        assert_eq!(LogSource::Stderr.as_str(), "STDERR");
    }

    #[test]
    fn test_log_entry_creation() {
        let entry = LogEntry {
            id: Some(1),
            timestamp: chrono::Utc::now(),
            container_id: "abc123".to_string(),
            container_name: "nginx".to_string(),
            compose_project: "myapp".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Server started".to_string(),
            level: LogLevel::Info,
            source: LogSource::Stdout,
            metadata: None,
        };

        assert_eq!(entry.id, Some(1));
        assert_eq!(entry.container_id, "abc123");
        assert_eq!(entry.level, LogLevel::Info);
    }

    #[test]
    fn test_log_query_result_with_data() {
        let entry = LogEntry {
            id: Some(1),
            timestamp: chrono::Utc::now(),
            container_id: "test".to_string(),
            container_name: "test".to_string(),
            compose_project: "test".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Test".to_string(),
            level: LogLevel::Info,
            source: LogSource::Stdout,
            metadata: None,
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

    #[test]
    fn test_log_entry_serialization() {
        let entry = LogEntry {
            id: Some(1),
            timestamp: chrono::Utc::now(),
            container_id: "test".to_string(),
            container_name: "test".to_string(),
            compose_project: "test".to_string(),
            compose_file: "docker-compose.yml".to_string(),
            message: "Test message".to_string(),
            level: LogLevel::Error,
            source: LogSource::Stderr,
            metadata: None,
        };

        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("test"));
        assert!(json.contains("Error"));
    }

    #[test]
    fn test_metric_entry_serialization() {
        let entry = MetricEntry {
            id: Some(1),
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
