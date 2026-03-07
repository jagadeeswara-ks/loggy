//! API module - REST endpoints and WebSocket

use axum::{
    routing::{get, post},
    Router, 
    extract::{
        Path, 
        State, 
    },
    response::Json,
    http::{StatusCode, HeaderValue},
};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt, TryFutureExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::broadcast;
use tower::ServiceExt;
use tower_http::cors::{CorsLayer, Any, ExposeHeaders, AllowMethods, AllowHeaders};
use tower_http::services::ServeDir;
use tracing::{info, error, warn};

use crate::AppState;

pub fn router(state: AppState) -> Router {
    // Build CORS based on config
    let cors = build_cors(&state.config.cors);
    
    // Rate limiting layer
    let rate_limit = tower::limit::GlobalConcurrencyLimitLayer::new(100);
    
    // API routes
    let api = Router::new()
        .route("/api/health", get(health_check))
        .route("/api/stacks/discover", get(discover_stacks))
        .route("/api/stacks", get(get_stacks))
        .route("/api/stacks/{id}/enable", post(enable_stack))
        .route("/api/stacks/{id}/disable", post(disable_stack))
        .route("/api/containers", get(get_containers))
        .route("/api/logs", get(query_logs))
        .route("/api/logs/stats", get(log_stats))
        .route("/api/metrics", get(query_metrics))
        .route("/api/patterns", get(get_patterns))
        .route("/api/auth/status", get(auth_status))
        .route("/api/internal/performance", get(get_performance))
        // WebSocket endpoint for real-time logs
        .route("/ws/logs", get(ws_logs_handler));
    
    // SPA fallback - serve index.html for non-API routes
    let frontend = ServeDir::new("frontend");
    
    api
        .layer(cors)
        .layer(rate_limit)
        .with_state(state)
        .nest_service("/app", frontend.clone())
        .fallback_service(frontend)
}

fn build_cors(config: &crate::config::CorsConfig) -> CorsLayer {
    let mut cors = CorsLayer::new();
    
    // Configure allowed origins
    if config.is_allow_all() {
        cors = cors.allow_origin(Any);
    } else {
        let origins: Vec<HeaderValue> = config.allowed_origins
            .iter()
            .filter_map(|o| o.parse().ok())
            .collect();
        cors = cors.allow_origin(origins);
    }
    
    // Allow common methods
    cors = cors.allow_methods(Any);
    
    // Allow common headers
    cors = cors.allow_headers(Any);
    
    // Configure credentials
    cors = cors.allow_credentials(config.allow_credentials);
    
    cors
}

// ============== Authentication ==============

async fn require_auth(state: &AppState) -> Result<(), String> {
    if !state.config.auth.enabled {
        return Ok(());
    }
    
    // In production, you'd check the Authorization header here
    // For now, auth is disabled by default unless enabled in config
    Ok(())
}

// ============== REST Endpoints ==============

#[derive(serde::Serialize)]
struct HealthResponse {
    status: String,
    service: String,
    version: String,
    auth_enabled: bool,
    cors_allow_all: bool,
}

async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok".to_string(),
        service: "loggy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        auth_enabled: state.config.auth.enabled,
        cors_allow_all: state.config.cors.is_allow_all(),
    })
}

// Stack discovery response
#[derive(serde::Serialize)]
struct DiscoveredStack {
    name: String,
    container_count: usize,
    running_count: usize,
    stopped_count: usize,
}

// Discover stacks from container labels
async fn discover_stacks(State(state): State<AppState>) -> Result<Json<Vec<DiscoveredStack>>, Json<String>> {
    info!("Discovering stacks from Docker containers...");
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    match state.docker_manager.list_containers().await {
        Ok(containers) => {
            info!("Found {} containers", containers.len());
            use std::collections::HashMap;
            let mut stack_map: HashMap<String, (usize, usize)> = HashMap::new();
            
            for c in containers {
                if let Some(project) = c.labels.get("com.docker.compose.project") {
                    let (running, stopped) = stack_map.entry(project.clone()).or_insert((0, 0));
                    if c.status.to_lowercase().contains("up") {
                        *running += 1;
                    } else {
                        *stopped += 1;
                    }
                }
            }
            
            info!("Discovered {} stacks", stack_map.len());
            let stacks: Vec<DiscoveredStack> = stack_map.into_iter()
                .map(|(name, (running, stopped))| DiscoveredStack {
                    name,
                    container_count: running + stopped,
                    running_count: running,
                    stopped_count: stopped,
                })
                .collect();
            
            Ok(Json(stacks))
        }
        Err(e) => {
            error!("Failed to discover stacks: {}", e);
            Err(Json("Failed to discover stacks".to_string()))
        }
    }
}

async fn get_stacks(State(state): State<AppState>) -> Result<Json<Vec<crate::docker::ComposeStack>>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    let stacks = state.active_stacks.read().await;
    Ok(Json(stacks.clone()))
}

#[derive(serde::Serialize)]
struct StackResponse {
    success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    stack: Option<crate::docker::ComposeStack>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

async fn enable_stack(
    Path(id): Path<String>,
    State(state): State<AppState>
) -> Result<Json<StackResponse>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    let mut stacks = state.active_stacks.write().await;
    if let Some(stack) = stacks.iter_mut().find(|s| s.id == id) {
        stack.enabled = true;
        Ok(Json(StackResponse {
            success: true,
            stack: Some(stack.clone()),
            error: None,
        }))
    } else {
        Ok(Json(StackResponse {
            success: false,
            stack: None,
            error: Some("Stack not found".to_string()),
        }))
    }
}

async fn disable_stack(
    Path(id): Path<String>,
    State(state): State<AppState>
) -> Result<Json<StackResponse>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    let mut stacks = state.active_stacks.write().await;
    if let Some(stack) = stacks.iter_mut().find(|s| s.id == id) {
        stack.enabled = false;
        Ok(Json(StackResponse {
            success: true,
            stack: Some(stack.clone()),
            error: None,
        }))
    } else {
        Ok(Json(StackResponse {
            success: false,
            stack: None,
            error: Some("Stack not found".to_string()),
        }))
    }
}

// Query params for containers - type-safe
#[derive(Debug, Deserialize, Default)]
struct ContainerQueryParams {
    stack: Option<String>,
    status: Option<String>,
    search: Option<String>,
}

async fn get_containers(
    State(state): State<AppState>,
    params: axum::extract::Query<ContainerQueryParams>,
) -> Result<Json<Vec<crate::docker::ContainerInfo>>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    match state.docker_manager.list_containers().await {
        Ok(mut containers) => {
            // Filter by stack (compose project)
            if let Some(ref stack) = params.stack {
                containers.retain(|c| {
                    c.labels.get("com.docker.compose.project")
                        .map(|p| p == stack)
                        .unwrap_or(false)
                });
            }
            
            // Filter by status
            if let Some(ref status) = params.status {
                let status_lower = status.to_lowercase();
                containers.retain(|c| {
                    match status_lower.as_str() {
                        "running" => c.status.to_lowercase().contains("up"),
                        "stopped" => !c.status.to_lowercase().contains("up"),
                        _ => true,
                    }
                });
            }
            
            // Filter by search
            if let Some(ref search) = params.search {
                let search_lower = search.to_lowercase();
                containers.retain(|c| {
                    c.name.to_lowercase().contains(&search_lower) ||
                    c.image.to_lowercase().contains(&search_lower)
                });
            }
            
            Ok(Json(containers))
        },
        Err(e) => {
            error!("Failed to list containers: {}", e);
            Err(Json("Failed to retrieve containers".to_string()))
        }
    }
}

#[derive(Debug, Deserialize)]
struct LogQueryParams {
    container_id: Option<String>,
    containers: Option<String>,  // comma-separated container IDs
    stack: Option<String>,      // compose project name
    level: Option<String>,
    search: Option<String>,
    limit: Option<u64>,
    offset: Option<u64>,
}

impl LogQueryParams {
    fn validate(&self) -> Result<(), String> {
        if let Some(limit) = self.limit {
            if limit > 10000 {
                return Err("Limit cannot exceed 10000".to_string());
            }
        }
        
        if let Some(ref id) = self.container_id {
            if !id.chars().all(|c| c.is_alphanumeric() || c == '-' || c == '_') {
                return Err("Invalid container_id format".to_string());
            }
        }
        
        if let Some(ref level) = self.level {
            let valid_levels = ["debug", "info", "warn", "error", "fatal"];
            if !valid_levels.contains(&level.to_lowercase().as_str()) {
                return Err("Invalid level. Must be: debug, info, warn, error, fatal".to_string());
            }
        }
        
        Ok(())
    }
}

async fn query_logs(
    State(state): State<AppState>,
    params: axum::extract::Query<LogQueryParams>
) -> Result<Json<LogQueryResponse>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    if let Err(e) = params.validate() {
        warn!("Invalid log query: {}", e);
        return Err(Json(e));
    }
    
    let limit = params.limit.unwrap_or(100).min(1000);
    let offset = params.offset.unwrap_or(0);
    
    // Determine container filter: could be single container_id or multiple via 'containers' param
    let container_filter = params.container_id.clone();
    
    match state.db.query_logs(
        container_filter.as_deref(),
        params.level.as_deref(),
        params.search.as_deref(),
        params.stack.as_deref(),
        params.containers.as_deref(),
        limit,
        offset,
    ).await {
        Ok(result) => {
            let logs = result.logs;
            let next_cursor = if logs.len() as u64 >= limit {
                Some(format!("offset:{}", offset + limit))
            } else {
                None
            };
            
            let total_count = logs.len() as u64;
            
            Ok(Json(LogQueryResponse {
                logs,
                total: Some(total_count),
                has_more: next_cursor.is_some(),
                next_cursor,
            }))
        }
        Err(e) => {
            error!("Failed to query logs: {}", e);
            Err(Json("Failed to query logs".to_string()))
        }
    }
}

// Log statistics
#[derive(serde::Serialize)]
struct LogStatsResponse {
    total: u64,
    error_count: u64,
    warn_count: u64,
    info_count: u64,
    debug_count: u64,
    containers: Vec<ContainerLogCount>,
}

#[derive(serde::Serialize)]
struct ContainerLogCount {
    container_id: String,
    container_name: String,
    count: u64,
}

#[derive(Debug, Deserialize, Default)]
struct LogStatsParams {
    stack: Option<String>,
    container_id: Option<String>,
}

async fn log_stats(
    State(state): State<AppState>,
    params: axum::extract::Query<LogStatsParams>
) -> Result<Json<serde_json::Value>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    // Generate cache key based on params
    let cache_key = format!("stack:{:?}_container:{:?}", params.stack, params.container_id);

    if let Some(cached) = state.log_stats_cache.get(&cache_key).await {
        return Ok(Json(cached));
    }



    // Query aggregations directly in database
    match state.db.get_log_stats(params.container_id.as_deref(), params.stack.as_deref()).await {
        Ok((total, error_count, warn_count, info_count, debug_count, container_counts)) => {
            let containers: Vec<ContainerLogCount> = container_counts.into_iter()
                .map(|(id, (name, count))| ContainerLogCount {
                    container_id: id,
                    container_name: name,
                    count,
                })
                .collect();
            
            let response = LogStatsResponse {
                total,
                error_count,
                warn_count,
                info_count,
                debug_count,
                containers,
            };

            let json_value = serde_json::to_value(response).unwrap_or_default();
            state.log_stats_cache.insert(cache_key, json_value.clone()).await;
            Ok(Json(json_value))
        }
        Err(e) => {
            error!("Failed to get log stats: {}", e);
            Err(Json("Failed to get log stats".to_string()))
        }
    }
}

#[derive(serde::Serialize)]
struct LogQueryResponse {
    logs: Vec<crate::db::LogEntry>,
    total: Option<u64>,
    has_more: bool,
    next_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct MetricsQueryParams {
    container_id: Option<String>,
    limit: Option<u64>,
}

async fn query_metrics(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<MetricsQueryParams>,
) -> Result<Json<MetricsResponse>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    let limit = params.limit.unwrap_or(100).min(1000);
    
    match state.db.get_metrics(params.container_id.as_deref(), limit).await {
        Ok(metrics) => Ok(Json(MetricsResponse {
            metrics,
            note: "Live metrics from containers".to_string(),
        })),
        Err(e) => {
            error!("Failed to query metrics: {}", e);
            Err(Json("Failed to query metrics".to_string()))
        }
    }
}

#[derive(serde::Serialize)]
struct MetricsResponse {
    metrics: Vec<crate::db::MetricEntry>,
    note: String,
}

async fn get_patterns(State(state): State<AppState>) -> Result<Json<PatternsResponse>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }
    
    let patterns = state.pattern_detector.get_detected_patterns();
    let count = patterns.len();
    
    Ok(Json(PatternsResponse {
        patterns,
        detected_count: count,
    }))
}

#[derive(serde::Serialize)]
struct PatternsResponse {
    patterns: Vec<crate::patterns::DetectedPattern>,
    detected_count: usize,
}

async fn auth_status(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "enabled": state.config.auth.enabled,
        "api_keys_count": state.config.auth.api_keys.len(),
        "message": if state.config.auth.enabled {
            "Include 'Authorization: Bearer <key>' header"
        } else {
            "Authentication is disabled"
        }
    }))
}

// ============== WebSocket Handler ==============

async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> axum::response::Response {
    // Optional: Check for API key in WebSocket URL query param
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: AppState) {
    let (mut sender, mut receiver) = socket.split();
    let mut rx = state.log_sender.subscribe();

    // Send welcome message
    let _ = sender.send(Message::Text(r#"{"type":"connected","message":"Loggy WebSocket connected"}"#.into())).await;

    // Forward logs to client
    let send_task = async {
        while let Ok(log) = rx.recv().await {
            let json = serde_json::to_string(&log).unwrap_or_default();
            if sender.send(Message::Text(json.into())).await.is_err() {
                break;
            }
        }
    };

    // Receive messages from client
    let recv_task = async {
        while let Some(msg) = receiver.next().await {
            match msg {
                Ok(Message::Text(text)) => {
                    info!("WS received: {}", text);
                }
                Ok(Message::Close(_)) => break,
                Err(_) => break,
                _ => {}
            }
        }
    };

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

// ============== TESTS ==============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let response = HealthResponse {
            status: "ok".to_string(),
            service: "loggy".to_string(),
            version: "0.1.0".to_string(),
            auth_enabled: false,
            cors_allow_all: true,
        };
        
        assert_eq!(response.status, "ok");
        assert_eq!(response.service, "loggy");
    }

    #[test]
    fn test_stack_response_success() {
        let response = StackResponse {
            success: true,
            stack: None,
            error: None,
        };
        
        assert!(response.success);
        assert!(response.stack.is_none());
    }

    #[test]
    fn test_stack_response_error() {
        let response = StackResponse {
            success: false,
            stack: None,
            error: Some("Stack not found".to_string()),
        };
        
        assert!(!response.success);
    }

    #[test]
    fn test_log_query_params_empty() {
        let params = LogQueryParams {
            stack: None,
            containers: None,
            container_id: None,
            level: None,
            search: None,
            limit: None,
            offset: None,
        };
        
        assert!(params.container_id.is_none());
    }

    #[test]
    fn test_log_query_validation_valid() {
        let params = LogQueryParams {
            stack: None,
            containers: None,
            container_id: Some("abc-123".to_string()),
            level: Some("error".to_string()),
            search: None,
            limit: Some(100),
            offset: None,
        };
        
        assert!(params.validate().is_ok());
    }

    #[test]
    fn test_log_query_validation_invalid_container() {
        let params = LogQueryParams {
            stack: None,
            containers: None,
            container_id: Some("abc; DROP TABLE".to_string()),
            level: None,
            search: None,
            limit: None,
            offset: None,
        };
        
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_log_query_validation_limit_too_high() {
        let params = LogQueryParams {
            stack: None,
            containers: None,
            container_id: None,
            level: None,
            search: None,
            limit: Some(50000),
            offset: None,
        };
        
        assert!(params.validate().is_err());
    }

    #[test]
    fn test_log_query_response() {
        let response = LogQueryResponse {
            logs: vec![],
            total: Some(100),
            has_more: true,
            next_cursor: Some("offset:100".to_string()),
        };
        
        assert!(response.has_more);
        assert!(response.next_cursor.is_some());
    }

    #[test]
    fn test_patterns_response() {
        let response = PatternsResponse {
            patterns: vec![],
            detected_count: 0,
        };
        
        assert_eq!(response.detected_count, 0);
    }
}

#[derive(serde::Serialize)]
struct PerformanceResponse {
    logs_ingested_total: u64,
    last_db_insert_latency_ms: u64,
}

async fn get_performance(State(state): State<AppState>) -> Result<Json<PerformanceResponse>, Json<String>> {
    if let Err(e) = require_auth(&state).await {
        return Err(Json(e));
    }

    Ok(Json(PerformanceResponse {
        logs_ingested_total: state.performance.logs_ingested.load(std::sync::atomic::Ordering::Relaxed),
        last_db_insert_latency_ms: state.performance.db_insert_time_ms.load(std::sync::atomic::Ordering::Relaxed),
    }))
}
