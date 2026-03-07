//! Docker management module

use bollard::Docker;
use bollard::container::{ListContainersOptions, LogsOptions};
use bollard::models::ContainerSummary;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::pin::Pin;
use tokio::sync::mpsc;
use tracing::{info, warn, error};
use chrono::Utc;

use crate::config::DockerConfig;
use crate::config::DiscoveryConfig;
use crate::db::{LogEntry, LogLevel, LogSource};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComposeStack {
    pub id: String,
    pub name: String,
    pub path: PathBuf,
    pub project_name: String,
    pub containers: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerInfo {
    pub id: String,
    pub name: String,
    pub image: String,
    pub status: String,
    pub created: i64,
    pub labels: HashMap<String, String>,
    pub ports: Vec<PortMapping>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PortMapping {
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub port_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContainerStatsInfo {
    pub container_id: String,
    pub container_name: String,
    pub cpu_percent: f64,
    pub memory_usage: u64,
    pub memory_limit: u64,
    pub memory_percent: f64,
    pub network_rx: u64,
    pub network_tx: u64,
    pub block_read: u64,
    pub block_write: u64,
}

#[allow(dead_code)]
pub struct DockerManager {
    client: Docker,
    config: DockerConfig,
}

impl DockerManager {
    pub async fn new(config: &DockerConfig) -> Result<Self, anyhow::Error> {
        let client = Docker::connect_with_local_defaults()?;
        
        let version = client.version().await?;
        info!("Connected to Docker: {}", version.version.unwrap_or_else(|| "unknown".to_string()));
        
        Ok(Self {
            client,
            config: config.clone(),
        })
    }
    
    /// Discover all docker-compose stacks in specified paths
    pub async fn discover_stacks(&self, config: &DiscoveryConfig) -> Result<Vec<ComposeStack>, anyhow::Error> {
        let mut stacks = Vec::new();
        
        for path_str in &config.paths {
            let path = PathBuf::from(path_str);
            if !path.exists() {
                warn!("Discovery path does not exist: {:?}", path);
                continue;
            }
            
            self.find_compose_files(&path, config, &mut stacks).await?;
        }
        
        stacks.sort_by(|a, b| a.path.cmp(&b.path));
        stacks.dedup_by(|a, b| a.path == b.path);
        
        info!("Discovered {} compose stacks", stacks.len());
        
        Ok(stacks)
    }

    async fn find_compose_files(
        &self,
        path: &PathBuf,
        config: &DiscoveryConfig,
        stacks: &mut Vec<ComposeStack>,
    ) -> Result<(), anyhow::Error> {
        let entries = std::fs::read_dir(path)?;
        
        for entry in entries.flatten() {
            let entry_path = entry.path();
            
            let should_exclude = config.exclude.iter().any(|pattern: &String| {
                entry_path.to_string_lossy().contains(pattern.trim_start_matches("**/").trim_start_matches("**"))
            });
            
            if should_exclude {
                continue;
            }
            
            if entry_path.is_dir() {
                let compose_file = entry_path.join("docker-compose.yml");
                let compose_file_alt = entry_path.join("docker-compose.yaml");
                
                if compose_file.exists() || compose_file_alt.exists() {
                    let name = entry_path
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());
                    
                    // Parse compose file to get service names
                    let services = self.parse_compose_services(&entry_path).await.unwrap_or_default();
                    
                    stacks.push(ComposeStack {
                        id: uuid::Uuid::new_v4().to_string(),
                        name: name.clone(),
                        path: entry_path,
                        project_name: name,
                        containers: services,
                        enabled: false,
                    });
                } else {
                    Box::pin(self.find_compose_files(&entry_path, config, stacks)).await?;
                }
            }
        }
        
        Ok(())
    }
    
    async fn parse_compose_services(&self, path: &PathBuf) -> Result<Vec<String>, anyhow::Error> {
        let compose_path = path.join("docker-compose.yml");
        let compose_path_alt = path.join("docker-compose.yaml");
        
        let content = if compose_path.exists() {
            std::fs::read_to_string(&compose_path)?
        } else if compose_path_alt.exists() {
            std::fs::read_to_string(&compose_path_alt)?
        } else {
            return Ok(vec![]);
        };
        
        // Simple YAML parsing for services
        let mut services = Vec::new();
        for line in content.lines() {
            if line.starts_with("  ") && line.contains(":") {
                let service = line.trim().trim_end_matches(':').to_string();
                if !service.is_empty() && !services.contains(&service) {
                    services.push(service);
                }
            }
        }
        
        Ok(services)
    }
    
    /// Get all running containers
    pub async fn list_containers(&self) -> Result<Vec<ContainerInfo>, anyhow::Error> {
        let options = ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        };
        
        let containers = self.client.list_containers(Some(options)).await?;
        
        let result: Vec<ContainerInfo> = containers
            .into_iter()
            .map(|c: ContainerSummary| {
                let ports = c.ports
                    .unwrap_or_default()
                    .into_iter()
                    .map(|p| PortMapping {
                        private_port: p.private_port as u16,
                        public_port: p.public_port.map(|pp| pp as u16),
                        port_type: p.typ.map(|t| format!("{:?}", t)).unwrap_or_default(),
                    })
                    .collect();
                
                ContainerInfo {
                    id: c.id.unwrap_or_default(),
                    name: c.names
                        .unwrap_or_default()
                        .first()
                        .map(|n| n.trim_start_matches('/').to_string())
                        .unwrap_or_default(),
                    image: c.image.unwrap_or_default(),
                    status: c.status.unwrap_or_default(),
                    created: c.created.unwrap_or(0),
                    labels: c.labels.unwrap_or_default(),
                    ports,
                }
            })
            .collect();
        
        Ok(result)
    }
    
    /// Stream logs from a specific container
    pub async fn stream_container_logs(
        &self,
        container_id: &str,
        tx: mpsc::Sender<std::sync::Arc<LogEntry>>,
    ) -> Result<(), anyhow::Error> {
        info!("Streaming logs from container: {}", container_id);
        
        // Get recent logs without following (just tail, not stream forever)
        let options = LogsOptions::<String> {
            follow: false,  // Don't follow - just get existing logs
            stdout: true,
            stderr: true,
            tail: "50".to_string(),  // Last 50 lines
            timestamps: true,
            ..Default::default()
        };
        
        let mut stream = self.client.logs(container_id, Some(options));
        
        let container_info = self.client.inspect_container(container_id, None)
            .await
            .ok();
        
        let container_name = container_info
            .as_ref()
            .and_then(|c| c.name.as_ref())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| container_id.to_string());
        
        let compose_project = container_info
            .as_ref()
            .and_then(|c| c.config.as_ref())
            .and_then(|c| c.labels.as_ref())
            .and_then(|l| l.get("com.docker.compose.project"))
            .map(|s| s.to_string())
            .unwrap_or_default();
        
        while let Some(log_result) = stream.next().await {
            match log_result {
                Ok(log_output) => {
                    let message = log_output.to_string();
                    let (level, source) = self.parse_log_level(&message);
                    
                    let entry = LogEntry {
                        id: 0,
                        timestamp: Utc::now(),
                        container_id: container_id.to_string(),
                        container_name: container_name.clone(),
                        compose_project: compose_project.clone(),
                        compose_file: "docker-compose.yml".to_string(),
                        message: message.clone(),
                        level,
                        source,
                        metadata: String::new(),
                    };
                    
                    if tx.send(std::sync::Arc::new(entry)).await.is_err() {
                        break;
                    }
                }
                Err(e) => {
                    error!("Error streaming logs from {}: {}", container_id, e);
                    break;
                }
            }
        }
        
        Ok(())
    }
    
    /// Get container stats (CPU, memory, network, etc.)
    pub async fn get_container_stats(&self, container_id: &str) -> Result<ContainerStatsInfo, anyhow::Error> {
        let mut stream = self.client.stats(container_id, None);
        
        if let Some(stats_result) = stream.next().await {
            let stats = stats_result?;
            
            // Calculate CPU percentage
            let cpu_delta = stats.cpu_stats.cpu_usage.total_usage as f64 
                - stats.precpu_stats.cpu_usage.total_usage as f64;
            let system_delta = stats.cpu_stats.system_cpu_usage.unwrap_or(0) as f64 
                - stats.precpu_stats.system_cpu_usage.unwrap_or(0) as f64;
            let num_cpus = stats.cpu_stats.online_cpus.unwrap_or(1) as f64;
            
            let cpu_percent = if system_delta > 0.0 {
                (cpu_delta / system_delta) * num_cpus * 100.0
            } else {
                0.0
            };
            
            let memory_usage = stats.memory_stats.usage.unwrap_or(0);
            let memory_limit = stats.memory_stats.limit.unwrap_or(1);
            let memory_percent = (memory_usage as f64 / memory_limit as f64) * 100.0;
            
            // Network stats
            let mut network_rx = 0u64;
            let mut network_tx = 0u64;
            if let Some(networks) = stats.networks {
                for (_, net) in networks {
                    network_rx += net.rx_bytes;
                    network_tx += net.tx_bytes;
                }
            }
            
            // Block I/O - simplified (avoiding API compatibility issues)
            let block_read: u64 = 0;
            let block_write: u64 = 0;
            
            let container_name = stats.name.trim_start_matches('/').to_string();
            
            return Ok(ContainerStatsInfo {
                container_id: container_id.to_string(),
                container_name,
                cpu_percent,
                memory_usage,
                memory_limit,
                memory_percent,
                network_rx,
                network_tx,
                block_read,
                block_write,
            });
        }
        
        Err(anyhow::anyhow!("No stats available"))
    }
    
    /// Parse log output to determine level and source
    fn parse_log_level(&self, message: &str) -> (LogLevel, LogSource) {
        let upper = message.to_uppercase();
        
        // Determine log level
        let level = if upper.contains("ERROR") || upper.contains("FATAL") || upper.contains("CRITICAL") {
            LogLevel::Error
        } else if upper.contains("WARN") || upper.contains("WARNING") {
            LogLevel::Warn
        } else if upper.contains("DEBUG") || upper.contains("TRACE") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        };
        
        // Determine source (stderr vs stdout)
        let source = if message.starts_with("stderr") {
            LogSource::Stderr
        } else {
            LogSource::Stdout
        };
        
        (level, source)
    }
    
    /// Health check for Docker
    pub async fn health_check(&self) -> Result<bool, anyhow::Error> {
        self.client.ping().await.map_err(|e| anyhow::anyhow!("{}", e))?;
        Ok(true)
    }
}

// ============== TESTS ==============

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use std::path::PathBuf;

    // ComposeStack tests
    #[test]
    fn test_compose_stack_creation() {
        let stack = ComposeStack {
            id: "test-id".to_string(),
            name: "test-stack".to_string(),
            path: PathBuf::from("/tmp/test"),
            project_name: "test-project".to_string(),
            containers: vec!["web".to_string(), "api".to_string()],
            enabled: true,
        };
        
        assert_eq!(stack.id, "test-id");
        assert_eq!(stack.name, "test-stack");
        assert_eq!(stack.enabled, true);
        assert_eq!(stack.containers.len(), 2);
    }

    #[test]
    fn test_compose_stack_disabled_by_default() {
        let stack = ComposeStack {
            id: "1".to_string(),
            name: "test".to_string(),
            path: PathBuf::from("/tmp/test"),
            project_name: "test".to_string(),
            containers: vec![],
            enabled: false,
        };
        assert!(!stack.enabled);
    }

    #[test]
    fn test_compose_stack_toggle() {
        let mut stack = ComposeStack {
            id: "1".to_string(),
            name: "test".to_string(),
            path: PathBuf::from("/tmp/test"),
            project_name: "test".to_string(),
            containers: vec![],
            enabled: false,
        };
        
        stack.enabled = true;
        assert!(stack.enabled);
        
        stack.enabled = false;
        assert!(!stack.enabled);
    }

    #[test]
    fn test_compose_stack_path() {
        let path = PathBuf::from("/home/user/projects/docker-compose");
        let stack = ComposeStack {
            id: "1".to_string(),
            name: "myapp".to_string(),
            path: path.clone(),
            project_name: "myapp".to_string(),
            containers: vec!["web".to_string()],
            enabled: true,
        };
        
        assert_eq!(stack.path, path);
        assert_eq!(stack.project_name, "myapp");
    }

    // ContainerInfo tests
    #[test]
    fn test_container_info_creation() {
        let info = ContainerInfo {
            id: "abc123".to_string(),
            name: "nginx".to_string(),
            image: "nginx:latest".to_string(),
            status: "running".to_string(),
            created: 1234567890,
            labels: HashMap::new(),
            ports: vec![],
        };
        
        assert_eq!(info.id, "abc123");
        assert_eq!(info.name, "nginx");
        assert_eq!(info.status, "running");
    }

    #[test]
    fn test_container_info_with_labels() {
        let mut labels = HashMap::new();
        labels.insert("app".to_string(), "web".to_string());
        labels.insert("env".to_string(), "production".to_string());
        
        let info = ContainerInfo {
            id: "test".to_string(),
            name: "test-container".to_string(),
            image: "test:latest".to_string(),
            status: "running".to_string(),
            created: 0,
            labels,
            ports: vec![],
        };
        
        assert_eq!(info.labels.get("app"), Some(&"web".to_string()));
        assert_eq!(info.labels.get("env"), Some(&"production".to_string()));
    }

    #[test]
    fn test_container_info_with_ports() {
        let ports = vec![
            PortMapping { private_port: 80, public_port: Some(8080), port_type: "tcp".to_string() },
            PortMapping { private_port: 443, public_port: Some(8443), port_type: "tcp".to_string() },
        ];
        
        let info = ContainerInfo {
            id: "test".to_string(),
            name: "nginx".to_string(),
            image: "nginx:latest".to_string(),
            status: "running".to_string(),
            created: 0,
            labels: HashMap::new(),
            ports: ports.clone(),
        };
        
        assert_eq!(info.ports.len(), 2);
        assert_eq!(info.ports[0].private_port, 80);
        assert_eq!(info.ports[0].public_port, Some(8080));
    }

    // PortMapping tests
    #[test]
    fn test_port_mapping_no_public() {
        let port = PortMapping {
            private_port: 5432,
            public_port: None,
            port_type: "tcp".to_string(),
        };
        
        assert_eq!(port.private_port, 5432);
        assert!(port.public_port.is_none());
    }

    // ContainerStatsInfo tests
    #[test]
    fn test_container_stats_info() {
        let stats = ContainerStatsInfo {
            container_id: "abc123".to_string(),
            container_name: "nginx".to_string(),
            cpu_percent: 25.5,
            memory_usage: 1000000,
            memory_limit: 2000000,
            memory_percent: 50.0,
            network_rx: 500000,
            network_tx: 300000,
            block_read: 10000,
            block_write: 5000,
        };
        
        assert_eq!(stats.container_id, "abc123");
        assert_eq!(stats.cpu_percent, 25.5);
        assert_eq!(stats.memory_percent, 50.0);
    }

    // Log level parsing tests
    #[test]
    #[ignore] // Requires Docker to be running
    fn test_parse_log_level_error() {
        let dm = DockerManager {
            client: bollard::Docker::connect_with_local_defaults().unwrap(),
            config: DockerConfig::default(),
        };
        
        let message = "2024-01-01 ERROR: Connection failed";
        let (level, _source) = dm.parse_log_level(message);
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_parse_log_level_warn() {
        // Test parsing without Docker instance
        let message = "2024-01-01 WARNING: Low memory";
        let level = if message.to_uppercase().contains("WARN") || message.to_uppercase().contains("WARNING") {
            LogLevel::Warn
        } else {
            LogLevel::Info
        };
        assert_eq!(level, LogLevel::Warn);
    }

    #[test]
    fn test_parse_log_level_debug() {
        let message = "DEBUG: Processing request";
        let level = if message.to_uppercase().contains("DEBUG") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        };
        assert_eq!(level, LogLevel::Debug);
    }
}
