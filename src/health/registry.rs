use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};
use crate::observability::metrics;

/// Health status of an upstream service
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamHealth {
    /// Upstream is healthy and can receive traffic
    Healthy,
    /// Upstream is unhealthy and should be skipped
    Unhealthy,
}

/// Registry to track health status of all upstreams
#[derive(Clone, Debug)]
pub struct HealthRegistry {
    /// Map of upstream URL to health status
    status: Arc<RwLock<HashMap<String, UpstreamHealth>>>,
}

impl HealthRegistry {
    /// Create a new health registry
    pub fn new() -> Self {
        Self {
            status: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an upstream with initial healthy status
    pub async fn register(&self, upstream: String) {
        let mut status = self.status.write().await;
        status.insert(upstream.clone(), UpstreamHealth::Healthy);
        info!(upstream = %upstream, "Registered upstream for health tracking");
    }

    /// Mark an upstream as healthy
    pub async fn mark_healthy(&self, upstream: &str) {
        let mut status = self.status.write().await;
        
        if let Some(current) = status.get(upstream) {
            if *current == UpstreamHealth::Unhealthy {
                info!(upstream = %upstream, "Upstream recovered to healthy");
            }
        }
        
        status.insert(upstream.to_string(), UpstreamHealth::Healthy);
        metrics::record_upstream_health_change(upstream, "healthy");
    }

    /// Mark an upstream as unhealthy
    pub async fn mark_unhealthy(&self, upstream: &str, reason: &str) {
        let mut status = self.status.write().await;
        
        if let Some(current) = status.get(upstream) {
            if *current == UpstreamHealth::Healthy {
                warn!(
                    upstream = %upstream,
                    reason = %reason,
                    "Upstream marked as unhealthy"
                );
            }
        }
        
        status.insert(upstream.to_string(), UpstreamHealth::Unhealthy);
        metrics::record_upstream_health_change(upstream, "unhealthy");
    }

    /// Get the health status of an upstream
    pub async fn is_healthy(&self, upstream: &str) -> bool {
        let status = self.status.read().await;
        status
            .get(upstream)
            .map(|s| *s == UpstreamHealth::Healthy)
            .unwrap_or(true) // Default to healthy if not tracked
    }

    /// Get health status of all upstreams
    pub async fn get_all_status(&self) -> HashMap<String, UpstreamHealth> {
        self.status.read().await.clone()
    }

    /// Check if any upstreams are healthy
    pub async fn has_healthy_upstream(&self, upstreams: &[String]) -> bool {
        let status = self.status.read().await;
        upstreams.iter().any(|upstream| {
            status
                .get(upstream)
                .map(|s| *s == UpstreamHealth::Healthy)
                .unwrap_or(true)
        })
    }

    /// Get list of healthy upstreams from a set
    pub async fn get_healthy_upstreams(&self, upstreams: &[String]) -> Vec<String> {
        let status = self.status.read().await;
        upstreams
            .iter()
            .filter(|upstream| {
                status
                    .get(*upstream)
                    .map(|s| *s == UpstreamHealth::Healthy)
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }
}

impl Default for HealthRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_register_upstream() {
        let registry = HealthRegistry::new();
        registry.register("http://localhost:3001".to_string()).await;
        assert!(registry.is_healthy("http://localhost:3001").await);
    }

    #[tokio::test]
    async fn test_mark_unhealthy() {
        let registry = HealthRegistry::new();
        registry.register("http://localhost:3001".to_string()).await;
        registry
            .mark_unhealthy("http://localhost:3001", "timeout")
            .await;
        assert!(!registry.is_healthy("http://localhost:3001").await);
    }

    #[tokio::test]
    async fn test_mark_healthy() {
        let registry = HealthRegistry::new();
        registry.register("http://localhost:3001".to_string()).await;
        registry
            .mark_unhealthy("http://localhost:3001", "timeout")
            .await;
        registry.mark_healthy("http://localhost:3001").await;
        assert!(registry.is_healthy("http://localhost:3001").await);
    }

    #[tokio::test]
    async fn test_get_healthy_upstreams() {
        let registry = HealthRegistry::new();
        let upstreams = vec![
            "http://localhost:3001".to_string(),
            "http://localhost:3002".to_string(),
            "http://localhost:3003".to_string(),
        ];

        for upstream in &upstreams {
            registry.register(upstream.clone()).await;
        }

        registry
            .mark_unhealthy("http://localhost:3002", "timeout")
            .await;

        let healthy = registry.get_healthy_upstreams(&upstreams).await;
        assert_eq!(healthy.len(), 2);
        assert!(!healthy.contains(&"http://localhost:3002".to_string()));
    }
}
