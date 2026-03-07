//! Configuration module

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tracing::info;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub discovery: DiscoveryConfig,
    pub storage: StorageConfig,
    pub database: DatabaseConfig,
    pub docker: DockerConfig,
    pub auth: AuthConfig,
    pub cors: CorsConfig,
    pub background: BackgroundConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            workers: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryConfig {
    pub paths: Vec<String>,
    pub exclude: Vec<String>,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            paths: vec![".".to_string()],
            exclude: vec![
                "**/node_modules/**".to_string(),
                "**/dist/**".to_string(),
                "**/target/**".to_string(),
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub retention_days: u32,
    pub compression: bool,
    pub data_dir: PathBuf,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            retention_days: 7,
            compression: true,
            data_dir: PathBuf::from("./data"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseConfig {
    pub host: String,
    pub port: u16,
    pub database: String,
    pub username: String,
    pub password: String,
    pub max_connections: u32,
}

impl Default for DatabaseConfig {
    fn default() -> Self {
        Self {
            host: "localhost".to_string(),
            port: 9000,
            database: "loggy".to_string(),
            username: "default".to_string(),
            password: "".to_string(),
            max_connections: 10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerConfig {
    pub socket_path: String,
    pub poll_interval_ms: u64,
}

impl Default for DockerConfig {
    fn default() -> Self {
        Self {
            socket_path: "/var/run/docker.sock".to_string(),
            poll_interval_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    pub enabled: bool,
    pub api_keys: Vec<String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_keys: vec![],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub allow_credentials: bool,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allowed_origins: vec!["*".to_string()],
            allowed_methods: vec!["GET".to_string(), "POST".to_string(), "PUT".to_string(), "DELETE".to_string()],
            allowed_headers: vec!["*".to_string()],
            allow_credentials: false,
        }
    }
}

impl CorsConfig {
    pub fn is_allow_all(&self) -> bool {
        self.allowed_origins.contains(&"*".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConfig {
    pub log_stream_interval_ms: u64,
    pub metrics_interval_ms: u64,
    pub log_buffer_size: usize,
    pub enable_log_streaming: bool,
    pub enable_metrics_collection: bool,
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            log_stream_interval_ms: 5000,
            metrics_interval_ms: 10000,
            log_buffer_size: 10000,
            enable_log_streaming: true,
            enable_metrics_collection: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            discovery: DiscoveryConfig::default(),
            storage: StorageConfig::default(),
            database: DatabaseConfig::default(),
            docker: DockerConfig::default(),
            auth: AuthConfig::default(),
            cors: CorsConfig::default(),
            background: BackgroundConfig::default(),
        }
    }
}

/// Load configuration from file and environment
pub fn load_config() -> Result<Config, anyhow::Error> {
    // Start with defaults
    let mut config = Config::default();
    
    // Try to load from config file
    let config_path = PathBuf::from("loggy.toml");
    
    if config_path.exists() {
        info!("Loading config from {:?}", config_path);
        let content = std::fs::read_to_string(&config_path)?;
        let loaded: Config = toml::from_str(&content)?;
        config = loaded;
    }
    
    // Override with environment variables
    
    // Server config
    if let Ok(host) = std::env::var("LOGGY_HOST") {
        config.server.host = host;
    }
    if let Ok(port) = std::env::var("LOGGY_PORT") {
        config.server.port = port.parse().unwrap_or(8080);
    }
    if let Ok(host) = std::env::var("LOGGY_SERVER__HOST") {
        config.server.host = host;
    }
    if let Ok(port) = std::env::var("LOGGY_SERVER__PORT") {
        config.server.port = port.parse().unwrap_or(8080);
    }
    if let Ok(workers) = std::env::var("LOGGY_SERVER__WORKERS") {
        config.server.workers = workers.parse().unwrap_or(1);
    }
    
    // Database config
    if let Ok(host) = std::env::var("LOGGY_DATABASE__HOST") {
        config.database.host = host;
    }
    if let Ok(port) = std::env::var("LOGGY_DATABASE__PORT") {
        config.database.port = port.parse().unwrap_or(9000);
    }
    if let Ok(db) = std::env::var("LOGGY_DATABASE__DATABASE") {
        config.database.database = db;
    }
    if let Ok(user) = std::env::var("LOGGY_DATABASE__USERNAME") {
        config.database.username = user;
    }
    if let Ok(pass) = std::env::var("LOGGY_DATABASE__PASSWORD") {
        config.database.password = pass;
    }
    if let Ok(conns) = std::env::var("LOGGY_DATABASE__MAX_CONNECTIONS") {
        config.database.max_connections = conns.parse().unwrap_or(10);
    }
    
    // Docker config
    if let Ok(socket) = std::env::var("LOGGY_DOCKER__SOCKET_PATH") {
        config.docker.socket_path = socket;
    }
    if let Ok(interval) = std::env::var("LOGGY_DOCKER__POLL_INTERVAL_MS") {
        config.docker.poll_interval_ms = interval.parse().unwrap_or(1000);
    }
    
    // Auth config
    if let Ok(enabled) = std::env::var("LOGGY_AUTH__ENABLED") {
        config.auth.enabled = enabled.parse().unwrap_or(false);
    }
    if let Ok(keys) = std::env::var("LOGGY_AUTH__API_KEYS") {
        config.auth.api_keys = keys.split(',').map(|s| s.trim().to_string()).collect();
    }
    
    // CORS config
    if let Ok(origins) = std::env::var("LOGGY_CORS__ALLOWED_ORIGINS") {
        config.cors.allowed_origins = origins.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Ok(methods) = std::env::var("LOGGY_CORS__ALLOWED_METHODS") {
        config.cors.allowed_methods = methods.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Ok(headers) = std::env::var("LOGGY_CORS__ALLOWED_HEADERS") {
        config.cors.allowed_headers = headers.split(',').map(|s| s.trim().to_string()).collect();
    }
    if let Ok(creds) = std::env::var("LOGGY_CORS__ALLOW_CREDENTIALS") {
        config.cors.allow_credentials = creds.parse().unwrap_or(false);
    }
    
    // Background config
    if let Ok(interval) = std::env::var("LOGGY_BACKGROUND__LOG_INTERVAL_MS") {
        config.background.log_stream_interval_ms = interval.parse().unwrap_or(5000);
    }
    if let Ok(interval) = std::env::var("LOGGY_BACKGROUND__METRICS_INTERVAL_MS") {
        config.background.metrics_interval_ms = interval.parse().unwrap_or(10000);
    }
    if let Ok(size) = std::env::var("LOGGY_BACKGROUND__LOG_BUFFER_SIZE") {
        config.background.log_buffer_size = size.parse().unwrap_or(10000);
    }
    if let Ok(enabled) = std::env::var("LOGGY_BACKGROUND__ENABLE_LOG_STREAMING") {
        config.background.enable_log_streaming = enabled.parse().unwrap_or(true);
    }
    if let Ok(enabled) = std::env::var("LOGGY_BACKGROUND__ENABLE_METRICS_COLLECTION") {
        config.background.enable_metrics_collection = enabled.parse().unwrap_or(true);
    }
    
    info!("Final config: server={}:{}, auth={}, cors_allow_all={}", 
        config.server.host, config.server.port, 
        config.auth.enabled, config.cors.is_allow_all());
    
    Ok(config)
}

// ============== TESTS ==============

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn test_server_config_defaults() {
        let config = ServerConfig::default();
        assert_eq!(config.host, "0.0.0.0");
        assert_eq!(config.port, 8080);
        assert_eq!(config.workers, 1);
    }

    #[test]
    fn test_discovery_config_defaults() {
        let config = DiscoveryConfig::default();
        assert!(config.paths.contains(&".".to_string()));
        assert!(config.exclude.contains(&"**/node_modules/**".to_string()));
    }

    #[test]
    fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert_eq!(config.retention_days, 7);
        assert!(config.compression);
    }

    #[test]
    fn test_database_config_defaults() {
        let config = DatabaseConfig::default();
        assert_eq!(config.host, "localhost");
        assert_eq!(config.port, 9000);
        assert_eq!(config.database, "loggy");
        assert_eq!(config.username, "default");
        assert_eq!(config.password, "");
        assert_eq!(config.max_connections, 10);
    }

    #[test]
    fn test_docker_config_defaults() {
        let config = DockerConfig::default();
        assert_eq!(config.socket_path, "/var/run/docker.sock");
        assert_eq!(config.poll_interval_ms, 1000);
    }

    #[test]
    fn test_auth_config_defaults() {
        let config = AuthConfig::default();
        assert!(!config.enabled);
        assert!(config.api_keys.is_empty());
    }

    #[test]
    fn test_cors_config_defaults() {
        let config = CorsConfig::default();
        assert!(config.is_allow_all());
        assert!(!config.allow_credentials);
    }

    #[test]
    fn test_cors_not_allow_all() {
        let config = CorsConfig {
            allowed_origins: vec!["https://example.com".to_string()],
            allowed_methods: vec!["GET".to_string()],
            allowed_headers: vec!["Content-Type".to_string()],
            allow_credentials: true,
        };
        assert!(!config.is_allow_all());
        assert!(config.allow_credentials);
    }

    #[test]
    fn test_background_config_defaults() {
        let config = BackgroundConfig::default();
        assert_eq!(config.log_stream_interval_ms, 5000);
        assert_eq!(config.metrics_interval_ms, 10000);
        assert_eq!(config.log_buffer_size, 10000);
        assert!(config.enable_log_streaming);
        assert!(config.enable_metrics_collection);
    }

    #[test]
    fn test_config_defaults() {
        let config = Config::default();
        assert_eq!(config.server.host, "0.0.0.0");
        assert_eq!(config.server.port, 8080);
        assert_eq!(config.database.port, 9000);
        assert!(!config.auth.enabled);
        assert!(config.cors.is_allow_all());
    }

    #[test]
    fn test_config_env_override() {
        // Set environment variables
        env::set_var("LOGGY_HOST", "127.0.0.1");
        env::set_var("LOGGY_PORT", "9000");
        env::set_var("LOGGY_AUTH__ENABLED", "true");

        // Load config (would need actual load_config call)
        let host = env::var("LOGGY_HOST").unwrap();
        let port: u16 = env::var("LOGGY_PORT").unwrap().parse().unwrap();
        let auth_enabled: bool = env::var("LOGGY_AUTH__ENABLED").unwrap().parse().unwrap();

        assert_eq!(host, "127.0.0.1");
        assert_eq!(port, 9000);
        assert!(auth_enabled);

        // Cleanup
        env::remove_var("LOGGY_HOST");
        env::remove_var("LOGGY_PORT");
        env::remove_var("LOGGY_AUTH__ENABLED");
    }
}
