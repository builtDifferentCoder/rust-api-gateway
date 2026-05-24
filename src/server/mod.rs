pub mod http_server;

use crate::config::loader::load_config;
use crate::router::router::build_router;
use crate::middleware;
use crate::observability;
use crate::health::{HealthRegistry, HealthChecker, HealthCheckConfig};
use axum::middleware::from_fn;
use axum::{extract::Request, middleware::Next};
use std::sync::Arc;
use std::time::Instant;
use tracing::info;

/// Initialize observability: metrics and tracing
fn init_observability() {
    observability::init_metrics();
    observability::init_structured_tracing();
}

/// Metrics middleware that records request duration and response status
async fn metrics_middleware(
    req: Request,
    next: Next,
) -> axum::response::Response {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    let start = Instant::now();

    // Execute the request
    let response = next.run(req).await;

    // Record metrics
    let duration = start.elapsed().as_secs_f64();
    let status = response.status().as_u16();

    observability::record_request(&method.to_string(), &path, status, duration);

    info!(
        method = %method,
        path = %path,
        status = status,
        duration_ms = duration * 1000.0,
        "Request completed"
    );

    response
}

/// Start health check background tasks for all routes
fn start_health_checks(
    config: &crate::config::loader::Config,
    health_registry: Arc<HealthRegistry>,
    shutdown_tx: &tokio::sync::broadcast::Sender<()>,
) {
    if config.routes.is_empty() {
        return;
    }

    let health_interval = config.health.as_ref().map(|h| h.interval_seconds).unwrap_or(10);
    let check_config = HealthCheckConfig::new(health_interval);

    for route in &config.routes {
        let checker = HealthChecker::new(check_config.clone(), health_registry.clone());
        let upstreams = route.upstreams.clone();
        let health_path = route.health_path.clone();
        let  rx = shutdown_tx.subscribe();

        // Spawn health check task for this route
        tokio::spawn(async move {
            checker.check_loop(upstreams, health_path, rx).await;
        });
    }

    info!(
        interval_seconds = health_interval,
        "Health check background tasks started"
    );
}

pub async fn run_server() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    init_observability();

    let config = load_config();

    // Create health registry for upstream health tracking
    let health_registry = Arc::new(HealthRegistry::new());

    // Shutdown broadcaster for background tasks
    let (shutdown_tx, _shutdown_rx) = tokio::sync::broadcast::channel::<()>(1);

    // Start health check background tasks if routes are configured
    if !config.routes.is_empty() {
        start_health_checks(&config, health_registry.clone(), &shutdown_tx);
    }

    // Create rate limiter if configured
    let rate_limiter = config.rate_limit.as_ref().map(|rl_config| {
        Arc::new(middleware::RateLimiter::new(middleware::RateLimitConfig::new(
            rl_config.requests_per_minute,
        )))
    });

    let rate_limiter_clone = rate_limiter.clone();

    let app = build_router(&config, Some(health_registry.clone()))
        // Apply metrics middleware (must be early to capture full request lifecycle)
        .layer(from_fn(metrics_middleware))
        // Apply rate limiting middleware (global protection)
        .layer(from_fn(move |mut req: axum::extract::Request, next: axum::middleware::Next| {
            let rl = rate_limiter_clone.clone();
            async move {
                if let Some(limiter) = rl {
                    // Add rate limiter to request extensions
                    req.extensions_mut().insert((*limiter).clone());
                    middleware::rate_limit_middleware_v2(req, next).await
                } else {
                    // Skip rate limiting if not configured
                    next.run(req).await
                }
            }
        }))
        // Apply conditional JWT authentication middleware
        .layer(from_fn(conditional_jwt_middleware))
        // Apply request logging middleware
        .layer(middleware::request_logger_layer());

    // Start HTTP server with graceful shutdown that notifies background tasks
    let address = format!("{}:{}", config.host, config.port);
    let listener = tokio::net::TcpListener::bind(&address).await?;
    info!(%address, "Starting HTTP server");

    let server_future = axum::serve(listener, app);

    tokio::select! {
        res = server_future => {
            res?;
        }
        _ = tokio::signal::ctrl_c() => {
            info!("Shutdown signal received, notifying background tasks");
            let _ = shutdown_tx.send(());
        }
    }

    Ok(())
}

/// Conditional JWT middleware that applies auth only to protected routes
async fn conditional_jwt_middleware(
    req: axum::extract::Request,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, axum::http::StatusCode> {
    // Check if this is a protected route
    let path = req.uri().path().to_string();
    
    if is_protected_route(&path) {
        // Apply JWT validation for protected routes
        middleware::jwt_auth_middleware(req.headers().clone(), req, next).await
    } else {
        // Skip JWT validation for public routes
        Ok(next.run(req).await)
    }
}

/// Check if a route path is protected (requires JWT authentication)
fn is_protected_route(path: &str) -> bool {
    // Define which routes require authentication
    // /metrics is public
    if path == "/metrics" {
        return false;
    }
    path.starts_with("/users")
}
