use std::sync::{atomic::{AtomicUsize, Ordering}, Arc};
use crate::health::HealthRegistry;

#[derive(Debug, Clone)]
pub struct RoundRobin {
    upstreams: Arc<Vec<String>>,
    index: Arc<AtomicUsize>,
    health_registry: Option<Arc<HealthRegistry>>,
}

impl RoundRobin {
    pub fn new(upstreams: Vec<String>) -> Self {
        Self {
            upstreams: Arc::new(upstreams),
            index: Arc::new(AtomicUsize::new(0)),
            health_registry: None,
        }
    }

    /// Create with health registry for health-aware load balancing
    pub fn with_health_registry(upstreams: Vec<String>, health_registry: Arc<HealthRegistry>) -> Self {
        Self {
            upstreams: Arc::new(upstreams),
            index: Arc::new(AtomicUsize::new(0)),
            health_registry: Some(health_registry),
        }
    }

    /// Get the next upstream, considering health status if available
    pub async fn next_async(&self) -> Option<String> {
        let len = self.upstreams.len();
        if len == 0 {
            return None;
        }

        // If health registry is available, only select from healthy upstreams
        if let Some(registry) = &self.health_registry {
            let healthy = registry.get_healthy_upstreams(&self.upstreams).await;
            
            if healthy.is_empty() {
                // No healthy upstreams available
                return None;
            }

            // Round-robin through healthy upstreams
            let current = self.index.fetch_add(1, Ordering::Relaxed);
            let selected = current % healthy.len();
            return healthy.get(selected).cloned();
        }

        // Fallback to original behavior if no health registry
        let current = self.index.fetch_add(1, Ordering::Relaxed);
        let selected = current % len;
        self.upstreams.get(selected).cloned()
    }

    /// Synchronous version for backward compatibility
    pub fn next(&self) -> Option<String> {
        let len = self.upstreams.len();
        if len == 0 {
            return None;
        }

        let current = self.index.fetch_add(1, Ordering::Relaxed);
        let selected = current % len;
        self.upstreams.get(selected).cloned()
    }
}
