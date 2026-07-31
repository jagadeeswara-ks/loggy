//! Alerts module

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alert {
    pub id: String,
    pub name: String,
    pub alert_type: AlertType,
    pub condition: AlertCondition,
    pub enabled: bool,
    pub triggered_count: u64,
    pub last_triggered: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum AlertType {
    Pattern,
    Threshold,
    ContainerRestart,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCondition {
    pub pattern: Option<String>,
    pub threshold_value: Option<f64>,
    pub container_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    pub alert_id: String,
    pub timestamp: DateTime<Utc>,
    pub message: String,
    pub container_id: Option<String>,
}

pub struct AlertManager {
    alerts: HashMap<String, Alert>,
    event_history: Vec<AlertEvent>,
    max_history: usize,
}

impl AlertManager {
    pub fn new() -> Self {
        Self {
            alerts: HashMap::new(),
            event_history: Vec::new(),
            max_history: 100,
        }
    }
    
    pub fn add_alert(&mut self, alert: Alert) {
        self.alerts.insert(alert.id.clone(), alert);
    }
    
    pub fn remove_alert(&mut self, id: &str) -> Option<Alert> {
        self.alerts.remove(id)
    }
    
    pub fn get_alerts(&self) -> Vec<&Alert> {
        self.alerts.values().collect()
    }
    
    pub fn get_enabled_alerts(&self) -> Vec<Alert> {
        self.alerts
            .values()
            .filter(|a| a.enabled)
            .cloned()
            .collect()
    }
    
    pub fn check_pattern_alert(&mut self, message: &str, container_id: &str) -> Option<AlertEvent> {
        let enabled_alerts = self.get_enabled_alerts();
        
        for alert in enabled_alerts {
            if alert.alert_type == AlertType::Pattern {
                if let Some(pattern) = &alert.condition.pattern {
                    if message.contains(pattern) {
                        let event = AlertEvent {
                            alert_id: alert.id.clone(),
                            timestamp: Utc::now(),
                            message: format!("Pattern '{}' detected in container {}", pattern, container_id),
                            container_id: Some(container_id.to_string()),
                        };
                        
                        self.record_event(event.clone());
                        
                        // Update alert stats
                        if let Some(a) = self.alerts.get_mut(&alert.id) {
                            a.triggered_count += 1;
                            a.last_triggered = Some(Utc::now());
                        }
                        
                        return Some(event);
                    }
                }
            }
        }
        None
    }
    
    fn record_event(&mut self, event: AlertEvent) {
        self.event_history.push(event);
        
        // Trim history if needed
        if self.event_history.len() > self.max_history {
            self.event_history.remove(0);
        }
    }
    
    pub fn get_recent_events(&self, count: usize) -> Vec<&AlertEvent> {
        self.event_history
            .iter()
            .rev()
            .take(count)
            .collect()
    }
    
    /// Create default alerts for common scenarios
    pub fn create_default_alerts(&mut self) {
        let defaults = vec![
            Alert {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Error Detection".to_string(),
                alert_type: AlertType::Pattern,
                condition: AlertCondition {
                    pattern: Some("ERROR".to_string()),
                    threshold_value: None,
                    container_id: None,
                },
                enabled: true,
                triggered_count: 0,
                last_triggered: None,
                created_at: Utc::now(),
            },
            Alert {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Exception Detection".to_string(),
                alert_type: AlertType::Pattern,
                condition: AlertCondition {
                    pattern: Some("exception".to_string()),
                    threshold_value: None,
                    container_id: None,
                },
                enabled: true,
                triggered_count: 0,
                last_triggered: None,
                created_at: Utc::now(),
            },
            Alert {
                id: uuid::Uuid::new_v4().to_string(),
                name: "Container Restart".to_string(),
                alert_type: AlertType::ContainerRestart,
                condition: AlertCondition {
                    pattern: Some("restarting".to_string()),
                    threshold_value: None,
                    container_id: None,
                },
                enabled: true,
                triggered_count: 0,
                last_triggered: None,
                created_at: Utc::now(),
            },
        ];
        
        for alert in defaults {
            self.alerts.insert(alert.id.clone(), alert);
        }
    }
}

impl Default for AlertManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============== TESTS ==============

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_alert_manager() {
        let manager = AlertManager::new();
        assert!(manager.get_alerts().is_empty());
    }

    #[test]
    fn test_add_alert() {
        let mut manager = AlertManager::new();
        let alert = Alert {
            id: "test-1".to_string(),
            name: "Test Alert".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("ERROR".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert);
        assert_eq!(manager.get_alerts().len(), 1);
    }

    #[test]
    fn test_remove_alert() {
        let mut manager = AlertManager::new();
        let alert = Alert {
            id: "test-1".to_string(),
            name: "Test Alert".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("ERROR".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert);
        assert_eq!(manager.get_alerts().len(), 1);
        
        let removed = manager.remove_alert("test-1");
        assert!(removed.is_some());
        assert_eq!(manager.get_alerts().len(), 0);
    }

    #[test]
    fn test_get_enabled_alerts() {
        let mut manager = AlertManager::new();
        
        let alert1 = Alert {
            id: "enabled".to_string(),
            name: "Enabled Alert".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("ERROR".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        let alert2 = Alert {
            id: "disabled".to_string(),
            name: "Disabled Alert".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("WARN".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: false,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert1);
        manager.add_alert(alert2);
        
        let enabled = manager.get_enabled_alerts();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].id, "enabled");
    }

    #[test]
    fn test_check_pattern_alert_triggered() {
        let mut manager = AlertManager::new();
        let alert = Alert {
            id: "error-alert".to_string(),
            name: "Error Alert".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("ERROR".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert);
        
        let event = manager.check_pattern_alert("ERROR: connection failed", "container-1");
        
        assert!(event.is_some());
        assert_eq!(event.unwrap().container_id, Some("container-1".to_string()));
    }

    #[test]
    fn test_check_pattern_alert_not_triggered() {
        let mut manager = AlertManager::new();
        let alert = Alert {
            id: "error-alert".to_string(),
            name: "Error Alert".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("CRITICAL".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert);
        
        let event = manager.check_pattern_alert("INFO: server started", "container-1");
        
        assert!(event.is_none());
    }

    #[test]
    fn test_alert_triggered_count_incremented() {
        let mut manager = AlertManager::new();
        let alert = Alert {
            id: "test".to_string(),
            name: "Test".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("ERROR".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert);
        
        // Trigger the alert multiple times
        manager.check_pattern_alert("ERROR: first", "container-1");
        manager.check_pattern_alert("ERROR: second", "container-1");
        manager.check_pattern_alert("ERROR: third", "container-1");
        
        let alerts = manager.get_alerts();
        let triggered_alert = alerts.iter().find(|a| a.id == "test").unwrap();
        assert_eq!(triggered_alert.triggered_count, 3);
    }

    #[test]
    fn test_alert_last_triggered_updated() {
        let mut manager = AlertManager::new();
        let alert = Alert {
            id: "test".to_string(),
            name: "Test".to_string(),
            alert_type: AlertType::Pattern,
            condition: AlertCondition {
                pattern: Some("ERROR".to_string()),
                threshold_value: None,
                container_id: None,
            },
            enabled: true,
            triggered_count: 0,
            last_triggered: None,
            created_at: Utc::now(),
        };
        
        manager.add_alert(alert);
        
        let before = Utc::now();
        manager.check_pattern_alert("ERROR: failed", "container-1");
        let after = Utc::now();
        
        let alerts = manager.get_alerts();
        let triggered_alert = alerts.iter().find(|a| a.id == "test").unwrap();
        
        assert!(triggered_alert.last_triggered.is_some());
        let triggered = triggered_alert.last_triggered.unwrap();
        assert!(triggered >= before && triggered <= after);
    }

    #[test]
    fn test_create_default_alerts() {
        let mut manager = AlertManager::new();
        manager.create_default_alerts();
        
        let alerts = manager.get_alerts();
        assert!(!alerts.is_empty());
        
        // Should have error detection, exception detection, and container restart
        let names: Vec<&str> = alerts.iter().map(|a| a.name.as_str()).collect();
        assert!(names.contains(&"Error Detection"));
        assert!(names.contains(&"Exception Detection"));
        assert!(names.contains(&"Container Restart"));
    }

    #[test]
    fn test_get_recent_events() {
        let mut manager = AlertManager::new();
        
        // Add some events
        for i in 0..5 {
            manager.record_event(AlertEvent {
                alert_id: "test".to_string(),
                timestamp: Utc::now(),
                message: format!("Event {}", i),
                container_id: Some("container-1".to_string()),
            });
        }
        
        let recent = manager.get_recent_events(3);
        assert_eq!(recent.len(), 3);
    }
}
