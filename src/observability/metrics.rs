use prometheus::{
    IntCounterVec, HistogramVec, Registry,
};
use std::sync::Arc;
use once_cell::sync::Lazy;

/// Global metrics registry
static METRICS_REGISTRY: Lazy<Registry> = Lazy::new(Registry::new);

/// Total number of HTTP requests received
pub static HTTP_REQUESTS_TOTAL: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new("http_requests_total", "Total HTTP requests received")
            .namespace("api_gateway"),
        &["method", "path", "status"],
    )
    .expect("Failed to create http_requests_total metric");

    METRICS_REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register http_requests_total");

    counter
});

/// Request duration histogram in seconds
pub static HTTP_REQUEST_DURATION_SECONDS: Lazy<HistogramVec> = Lazy::new(|| {
    let histogram = HistogramVec::new(
        prometheus::HistogramOpts::new(
            "http_request_duration_seconds",
            "HTTP request duration in seconds",
        )
        .namespace("api_gateway")
        .buckets(vec![0.001, 0.01, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0]),
        &["method", "path"],
    )
    .expect("Failed to create http_request_duration_seconds metric");

    METRICS_REGISTRY
        .register(Box::new(histogram.clone()))
        .expect("Failed to register http_request_duration_seconds");

    histogram
});

/// HTTP response status code distribution
pub static HTTP_RESPONSE_STATUS: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new(
            "http_response_status",
            "HTTP response status code counts",
        )
        .namespace("api_gateway"),
        &["status"],
    )
    .expect("Failed to create http_response_status metric");

    METRICS_REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register http_response_status");

    counter
});

/// Requests per route
pub static HTTP_REQUESTS_PER_ROUTE: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new("http_requests_per_route", "Requests per route")
            .namespace("api_gateway"),
        &["route"],
    )
    .expect("Failed to create http_requests_per_route metric");

    METRICS_REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register http_requests_per_route");

    counter
});

/// Upstream health state changes (labels: upstream, status)
pub static UPSTREAM_HEALTH_CHANGES: Lazy<IntCounterVec> = Lazy::new(|| {
    let counter = IntCounterVec::new(
        prometheus::Opts::new(
            "upstream_health_changes_total",
            "Upstream health state change events",
        )
        .namespace("api_gateway"),
        &["upstream", "status"],
    )
    .expect("Failed to create upstream_health_changes_total metric");

    METRICS_REGISTRY
        .register(Box::new(counter.clone()))
        .expect("Failed to register upstream_health_changes_total");

    counter
});

/// Metrics collector for recording request telemetry
#[derive(Clone)]
pub struct MetricsCollector;

impl MetricsCollector {
    /// Record a completed HTTP request
    pub fn record_request(
        method: &str,
        path: &str,
        status: u16,
        duration_seconds: f64,
    ) {
        // Record total request count
        HTTP_REQUESTS_TOTAL
            .with_label_values(&[method, path, &status.to_string()])
            .inc();

        // Record response status
        HTTP_RESPONSE_STATUS
            .with_label_values(&[&status.to_string()])
            .inc();

        // Record request duration
        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&[method, path])
            .observe(duration_seconds);

        // Record requests per route
        HTTP_REQUESTS_PER_ROUTE
            .with_label_values(&[path])
            .inc();
    }

    /// Record request duration for a specific route
    pub fn record_duration(method: &str, path: &str, duration_seconds: f64) {
        HTTP_REQUEST_DURATION_SECONDS
            .with_label_values(&[method, path])
            .observe(duration_seconds);
    }
}

/// Initialize metrics system
pub fn init_metrics() {
    // Force initialization of lazy statics
    let _ = HTTP_REQUESTS_TOTAL.clone();
    let _ = HTTP_REQUEST_DURATION_SECONDS.clone();
    let _ = HTTP_RESPONSE_STATUS.clone();
    let _ = HTTP_REQUESTS_PER_ROUTE.clone();

    println!("Prometheus metrics initialized");
}

/// Record upstream health change event
pub fn record_upstream_health_change(upstream: &str, status: &str) {
    UPSTREAM_HEALTH_CHANGES
        .with_label_values(&[upstream, status])
        .inc();
}

/// Get the global metrics registry
pub fn get_metrics_registry() -> Arc<Registry> {
    Arc::new(METRICS_REGISTRY.clone())
}

/// Encode metrics in Prometheus text format
pub fn encode_metrics() -> Result<String, prometheus::Error> {
    let encoder = prometheus::TextEncoder::new();
    let metric_families = METRICS_REGISTRY.gather();
    encoder.encode_to_string(&metric_families)
}

/// Record a request with all metrics
pub fn record_request(
    method: &str,
    path: &str,
    status: u16,
    duration_seconds: f64,
) {
    MetricsCollector::record_request(method, path, status, duration_seconds);
}

/// Record just the request duration
pub fn record_request_duration(method: &str, path: &str, duration_seconds: f64) {
    MetricsCollector::record_duration(method, path, duration_seconds);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        init_metrics();
        // If this doesn't panic, initialization succeeded
        assert!(true);
    }

    #[test]
    fn test_record_request() {
        init_metrics();
        record_request("GET", "/test", 200, 0.05);
        // Verify metrics were recorded (encode should not fail)
        let result = encode_metrics();
        assert!(result.is_ok());
        let metrics_text = result.unwrap();
        assert!(metrics_text.contains("http_requests_total"));
    }

    #[test]
    fn test_encode_metrics() {
        init_metrics();
        let result = encode_metrics();
        assert!(result.is_ok());
        let metrics_text = result.unwrap();
        assert!(metrics_text.contains("api_gateway_http_requests_total") || metrics_text.contains("http_requests_total"));
    }
}
