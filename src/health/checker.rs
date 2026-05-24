use crate::health::registry::HealthRegistry;
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;
use tracing::debug;

/// Configuration for health checks
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    /// Interval between health checks (in seconds)
    pub interval_seconds: u64,
    /// Timeout for each health check request (in seconds)
    pub timeout_seconds: u64,
}

impl HealthCheckConfig {
    /// Create a new health check configuration
    pub fn new(interval_seconds: u64) -> Self {
        Self {
            interval_seconds,
            timeout_seconds: 5,
        }
    }

    /// Create with custom timeout
    pub fn with_timeout(interval_seconds: u64, timeout_seconds: u64) -> Self {
        Self {
            interval_seconds,
            timeout_seconds,
        }
    }
}

/// Performs periodic health checks on upstream services
pub struct HealthChecker {
    config: HealthCheckConfig,
    registry: Arc<HealthRegistry>,
    client: Client,
}

impl HealthChecker {
    /// Create a new health checker
    pub fn new(config: HealthCheckConfig, registry: Arc<HealthRegistry>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(config.timeout_seconds))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            config,
            registry,
            client,
        }
    }

    /// Start periodic health checks for a set of upstreams
    /// This runs until a shutdown signal is received. Spawn as background task.
    pub async fn check_loop(
        &self,
        upstreams: Vec<String>,
        health_path: String,
        mut shutdown: tokio::sync::broadcast::Receiver<()>,
    ) {
        // Register all upstreams
        for upstream in &upstreams {
            self.registry.register(upstream.clone()).await;
        }

        loop {
            for upstream in &upstreams {
                tokio::select! {
                    _ = shutdown.recv() => {
                        debug!("Health checker shutting down for upstreams");
                        return;
                    }
                    _ = self.check_upstream(upstream, &health_path) => {}
                }
            }

            tokio::select! {
                _ = shutdown.recv() => {
                    debug!("Health checker shutting down after interval");
                    return;
                }
                _ = sleep(Duration::from_secs(self.config.interval_seconds)) => {}
            }
        }
    }

    /// Check the health of a single upstream
    async fn check_upstream(&self, upstream: &str, health_path: &str) {
        let url = format!("{}{}", upstream, health_path);

        match self.client.get(&url).send().await {
            Ok(response) => {
                if response.status().is_success() {
                    debug!(upstream = %upstream, "Upstream health check passed");
                    self.registry.mark_healthy(upstream).await;
                } else {
                    let status = response.status();
                    debug!(
                        upstream = %upstream,
                        status = %status,
                        "Upstream health check failed: non-success status"
                    );
                    self.registry
                        .mark_unhealthy(upstream, &format!("HTTP {}", status))
                        .await;
                }
            }
            Err(err) => {
                let reason = if err.is_timeout() {
                    "timeout".to_string()
                } else if err.is_connect() {
                    "connection refused".to_string()
                } else {
                    err.to_string()
                };

                debug!(
                    upstream = %upstream,
                    reason = %reason,
                    "Upstream health check failed: {}", err
                );
                self.registry.mark_unhealthy(upstream, &reason).await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_check_config() {
        let config = HealthCheckConfig::new(10);
        assert_eq!(config.interval_seconds, 10);
        assert_eq!(config.timeout_seconds, 5);
    }

    #[test]
    fn test_health_check_config_with_timeout() {
        let config = HealthCheckConfig::with_timeout(10, 15);
        assert_eq!(config.interval_seconds, 10);
        assert_eq!(config.timeout_seconds, 15);
    }

    #[tokio::test]
    async fn test_health_checker_creation() {
        let config = HealthCheckConfig::new(10);
        let registry = Arc::new(HealthRegistry::new());
        let checker = HealthChecker::new(config, registry);
        assert_eq!(checker.config.interval_seconds, 10);
    }
}
