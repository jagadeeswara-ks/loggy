//! Loggy - Docker Observability Platform

pub mod config;
pub mod db;
pub mod docker;
pub mod api;
pub mod patterns;
pub mod alerts;

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{RwLock, broadcast};
use tokio::task::JoinHandle;
use tokio::time::interval;
use tracing::{info, error, warn};

use crate::db::{LogEntry, MetricEntry};

/// Application state shared across all handlers
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct PerformanceMetrics {
    pub logs_ingested: AtomicU64,
    pub db_insert_time_ms: AtomicU64,
}

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<config::Config>,
    pub db: Arc<db::Database>,
    pub docker_manager: Arc<docker::DockerManager>,
    pub pattern_detector: Arc<patterns::PatternDetector>,
    pub alert_manager: Arc<RwLock<alerts::AlertManager>>,
    pub active_stacks: Arc<RwLock<Vec<docker::ComposeStack>>>,
    pub log_sender: broadcast::Sender<LogEntry>,
    pub log_stats_cache: Arc<moka::future::Cache<String, serde_json::Value>>,
    pub performance: Arc<PerformanceMetrics>,
}

/// Background task handles for graceful shutdown
pub struct BackgroundTasks {
    pub handles: Vec<JoinHandle<()>>,
    pub shutdown_tx: Option<broadcast::Sender<()>>,
}

impl BackgroundTasks {
    /// Create new background tasks manager
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);
        Self {
            handles: Vec::new(),
            shutdown_tx: Some(shutdown_tx),
        }
    }
    
    /// Spawn a new background task
    pub fn spawn(&mut self, handle: JoinHandle<()>) {
        self.handles.push(handle);
    }
    
    /// Stop all background tasks gracefully
    pub async fn stop(&mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        
        // Wait for all tasks to complete
        for handle in self.handles.drain(..) {
            handle.abort();
        }
    }
}

impl Default for BackgroundTasks {
    fn default() -> Self {
        Self::new()
    }
}

/// Initialize the application
pub async fn init() -> Result<AppState, anyhow::Error> {
    // Load configuration
    let config = config::load_config()?;
    info!("Configuration loaded: server={}:{}", config.server.host, config.server.port);
    
    // Initialize database
    let db = match db::Database::new(&config.database).await {
        Ok(db) => {
            info!("Database initialized successfully");
            Arc::new(db)
        }
        Err(e) => {
            error!("Database initialization failed: {}", e);
            return Err(e);
        }
    };
    
    // Initialize Docker manager
    let docker_manager = match docker::DockerManager::new(&config.docker).await {
        Ok(dm) => {
            info!("Docker manager initialized successfully");
            Arc::new(dm)
        }
        Err(e) => {
            error!("Docker manager initialization failed: {}", e);
            return Err(e);
        }
    };
    
    // Discover initial stacks
    let stacks = match docker_manager.discover_stacks(&config.discovery).await {
        Ok(s) => {
            info!("Discovered {} compose stacks", s.len());
            s
        }
        Err(e) => {
            warn!("Stack discovery failed: {}", e);
            vec![]
        }
    };
    
    let active_stacks = Arc::new(RwLock::new(stacks));
    
    // Initialize pattern detector
    let pattern_detector = Arc::new(patterns::PatternDetector::new());
    info!("Pattern detector initialized");
    
    // Initialize alert manager
    let mut alert_manager = alerts::AlertManager::new();
    alert_manager.create_default_alerts();
    let alert_manager = Arc::new(RwLock::new(alert_manager));
    info!("Alert manager initialized");
    
    // Initialize broadcast channel for WebSocket log streaming
    let buffer_size = config.background.log_buffer_size;
    let (log_sender, _) = broadcast::channel(buffer_size);
    info!("Log broadcast channel initialized (capacity: {})", buffer_size);
    
    Ok(AppState {
        log_stats_cache: Arc::new(moka::future::Cache::builder().time_to_live(std::time::Duration::from_secs(5)).build()),
        performance: Arc::new(PerformanceMetrics::default()),
        config: Arc::new(config),
        db,
        docker_manager,
        pattern_detector,
        alert_manager,
        active_stacks,
        log_sender,
    })
}

/// Start background tasks for log collection and metrics
pub fn start_background_tasks(state: AppState) -> BackgroundTasks {
    let mut tasks = BackgroundTasks::new();
    let config = state.config.clone();
    
    // Log streaming task
    if config.background.enable_log_streaming {
        let log_state = state.clone();
        let handle = tokio::spawn(async move {
            start_log_streamer(log_state).await;
        });
        tasks.spawn(handle);
        info!("Log streaming task started (interval: {}ms)", config.background.log_stream_interval_ms);
    }
    
    // Metrics collection task
    if config.background.enable_metrics_collection {
        let metrics_state = state.clone();
        let handle = tokio::spawn(async move {
            start_metrics_collector(metrics_state).await;
        });
        tasks.spawn(handle);
        info!("Metrics collection task started (interval: {}ms)", config.background.metrics_interval_ms);
    }
    
    tasks
}

/// Stream logs from all enabled containers
async fn start_log_streamer(state: AppState) {
    let interval_ms = state.config.background.log_stream_interval_ms;
    let mut ticker = interval(Duration::from_millis(interval_ms));
    
    loop {
        ticker.tick().await;
        
        // Get list of containers
        let containers = match state.docker_manager.list_containers().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to list containers: {}", e);
                continue;
            }
        };
        
        // Stream logs from all running containers (for development)
        // TODO: Add stack filtering for production
        for container in containers.iter().filter(|c| c.status.contains("Up")) {
            let container_id = &container.id;
            
            if let Err(e) = stream_container_logs(&state, container_id).await {
                warn!("Failed to stream logs from {}: {}", container_id, e);
            }
        }
    }
}

async fn stream_container_logs(state: &AppState, container_id: &str) -> Result<(), anyhow::Error> {
    use tokio::sync::mpsc;
    
    let (tx, mut rx) = mpsc::channel::<LogEntry>(100);
    let container_id = container_id.to_string();
    
    // Start streaming in background
    let state_clone = state.clone();
    let cid = container_id.clone();
    tokio::spawn(async move {
        if let Err(e) = state_clone.docker_manager.stream_container_logs(&cid, tx).await {
            error!("Log stream error for {}: {}", cid, e);
        }
    });
    
    // Process logs in batches
    let mut batch = Vec::new();
    let mut last_insert = std::time::Instant::now();

    loop {
        // Use timeout to force insert even if buffer isn't full
        let recv_result = tokio::time::timeout(Duration::from_secs(2), rx.recv()).await;
        
        match recv_result {
            Ok(Some(log_entry)) => {
                // Detect patterns
                let detected = state.pattern_detector.detect(&log_entry.message);
                if !detected.is_empty() {
                    for pattern in detected {
                        state.pattern_detector.record_pattern(pattern);
                    }
                }

                // Check alerts
                {
                    let mut alert_manager = state.alert_manager.write().await;
                    if let Some(event) = alert_manager.check_pattern_alert(&log_entry.message, &log_entry.container_id) {
                        info!("Alert triggered: {}", event.message);
                    }
                }

                // Broadcast to WebSocket subscribers
                let _ = state.log_sender.send(log_entry.clone());

                batch.push(log_entry);
            }
            Ok(None) => {
                // Channel closed, insert any remaining logs
                if !batch.is_empty() {
                    let start = std::time::Instant::now();
                    let batch_len = batch.len() as u64;
                    if let Err(e) = state.db.insert_logs(&batch).await {
                        warn!("Failed to insert final log batch: {}", e);
                    } else {
                        state.performance.logs_ingested.fetch_add(batch_len, Ordering::Relaxed);
                        state.performance.db_insert_time_ms.store(start.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                }
                break;
            }
            Err(_) => {} // Timeout, proceed to check batch size
        }
        
        if batch.len() >= 100 || (!batch.is_empty() && last_insert.elapsed() >= Duration::from_secs(2)) {
            let start = std::time::Instant::now();
            let batch_len = batch.len() as u64;

            if let Err(e) = state.db.insert_logs(&batch).await {
                warn!("Failed to insert log batch: {}", e);
            } else {
                state.performance.logs_ingested.fetch_add(batch_len, Ordering::Relaxed);
                state.performance.db_insert_time_ms.store(start.elapsed().as_millis() as u64, Ordering::Relaxed);
            }

            batch.clear();
            last_insert = std::time::Instant::now();
        }
    }
    
    Ok(())
}

/// Collect metrics from all containers periodically
async fn start_metrics_collector(state: AppState) {
    let interval_ms = state.config.background.metrics_interval_ms;
    let mut ticker = interval(Duration::from_millis(interval_ms));
    
    loop {
        ticker.tick().await;
        
        let containers = match state.docker_manager.list_containers().await {
            Ok(c) => c,
            Err(e) => {
                warn!("Failed to list containers for metrics: {}", e);
                continue;
            }
        };
        
        for container in containers.iter().filter(|c| c.status.contains("Up")) {
            match state.docker_manager.get_container_stats(&container.id).await {
                Ok(stats) => {
                    let metric = MetricEntry {
                        id: None,
                        timestamp: chrono::Utc::now(),
                        container_id: stats.container_id,
                        container_name: stats.container_name,
                        cpu_percent: stats.cpu_percent as f32,
                        memory_percent: stats.memory_percent as f32,
                        memory_usage: stats.memory_usage,
                        network_rx: stats.network_rx,
                        network_tx: stats.network_tx,
                        block_read: stats.block_read,
                        block_write: stats.block_write,
                    };
                    
                    if let Err(e) = state.db.insert_metric(&metric).await {
                        warn!("Failed to insert metric: {}", e);
                    }
                }
                Err(e) => {
                    warn!("Failed to get stats for {}: {}", container.id, e);
                }
            }
        }
    }
}

/// Run the application
pub async fn run(state: AppState) -> Result<(), anyhow::Error> {
    // Start background tasks
    let tasks = Arc::new(tokio::sync::Mutex::new(start_background_tasks(state.clone())));
    
    let addr = format!("{}:{}", state.config.server.host, state.config.server.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    info!("Loggy listening on http://{}", addr);
    
    // Clone for shutdown handler
    let tasks_clone = tasks.clone();
    
    // Setup graceful shutdown
    let result = axum::serve(listener, api::router(state))
        .with_graceful_shutdown(async move {
            // Wait for shutdown signal
            tokio::signal::ctrl_c().await.ok();
            info!("Shutdown signal received, stopping background tasks...");
            let mut tasks = tasks_clone.lock().await;
            tasks.stop().await;
            info!("All background tasks stopped");
        })
        .await;
    
    if let Err(e) = result {
        error!("Server error: {}", e);
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::broadcast;

    #[test]
    fn test_background_tasks_struct() {
        let tasks = BackgroundTasks::new();
        assert!(tasks.handles.is_empty());
    }

    #[test]
    fn test_background_tasks_default() {
        let tasks = BackgroundTasks::default();
        assert!(tasks.handles.is_empty());
    }
}
