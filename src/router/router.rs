use axum::{
    body::Body, 
    extract::Request,
    http::StatusCode,
    middleware::Next,
    routing::{any, get}, 
    Router,
    response::IntoResponse,
    http::header,
    Json,
};
use serde_json::json;
use std::sync::Arc;
use crate::config::loader::Config as AppConfig;
use crate::load_balancer::RoundRobin;
use crate::proxy::proxy_request;
use crate::middleware::JwtConfig;
use crate::observability::metrics;
use crate::health::HealthRegistry;
use tracing::error;
use crate::error::AppError;

/// Handler for /metrics endpoint
async fn metrics_handler() -> impl IntoResponse {
    match metrics::encode_metrics() {
        Ok(metrics_text) => {
            (
                StatusCode::OK,
                [(header::CONTENT_TYPE, "text/plain; version=0.0.4")],
                metrics_text,
            )
                .into_response()
        }
        Err(_) => {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Failed to encode metrics",
            )
                .into_response()
        }
    }
}

pub fn build_router(config: &AppConfig, health_registry: Option<Arc<HealthRegistry>>) -> Router {
    let mut router = Router::new();

    // Add metrics endpoint (public, no auth required)
    router = router.route("/metrics", get(metrics_handler));

    // Create JWT config if provided, wrapped in Arc for sharing across handlers
    let jwt_config = config.jwt.as_ref().map(|jwt_cfg| {
        Arc::new(JwtConfig::new(
            jwt_cfg.secret.clone(),
            jwt_cfg.token_expiry_hours,
        ))
    });

    for route in &config.routes {
        let prefix = route.path.clone();
        
        // Create load balancer with health registry if available
        let balancer = if let Some(registry) = &health_registry {
            Arc::new(RoundRobin::with_health_registry(route.upstreams.clone(), registry.clone()))
        } else {
            Arc::new(RoundRobin::new(route.upstreams.clone()))
        };

        let health_registry_clone = health_registry.clone();

        // Exact route handler (without wildcard)
        let exact_prefix = prefix.clone();
        let exact_route = exact_prefix.clone();
        let exact_balancer = balancer.clone();
        let exact_health_registry = health_registry_clone.clone();
        
        router = router.route(
            &exact_route,
            any(move |req: Request<Body>| {
                let prefix = exact_prefix.clone();
                let balancer = exact_balancer.clone();
                let health_registry = exact_health_registry.clone();
                async move {
                    // Try to get an upstream (health-aware if registry available)
                    let upstream = if let Some(_registry) = &health_registry {
                        balancer.next_async().await
                    } else {
                        balancer.next()
                    };

                    let upstream = match upstream {
                        Some(url) => url,
                        None => {
                            // All upstreams unhealthy or unavailable
                            let error = json!({
                                "error": "service_unavailable",
                                "message": "All upstream services are unavailable"
                            });
                            return (
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(error),
                            ).into_response();
                        }
                    };

                    proxy_request(req, &upstream, &prefix)
                        .await
                        .map_err(|err| {
                            error!(%prefix, %upstream, %err, "proxy error");
                            AppError::BadGateway(err.to_string())
                        })
                        .map(|resp| resp.into_response())
                        .unwrap_or_else(|e: AppError| e.into_response())
                }
            }),
        );

        // Wildcard route handler (with path parameter)
        let wildcard_prefix = prefix.clone();
        let wildcard_route = format!("{}/*path", wildcard_prefix);
        let wildcard_balancer = balancer.clone();
        let wildcard_health_registry = health_registry_clone.clone();
        
        router = router.route(
            &wildcard_route,
            any(move |req: Request<Body>| {
                let prefix = wildcard_prefix.clone();
                let balancer = wildcard_balancer.clone();
                let health_registry = wildcard_health_registry.clone();
                async move {
                    // Try to get an upstream (health-aware if registry available)
                    let upstream = if let Some(_registry) = &health_registry {
                        balancer.next_async().await
                    } else {
                        balancer.next()
                    };

                    let upstream = match upstream {
                        Some(url) => url,
                        None => {
                            // All upstreams unhealthy or unavailable
                            let error = json!({
                                "error": "service_unavailable",
                                "message": "All upstream services are unavailable"
                            });
                            return (
                                StatusCode::SERVICE_UNAVAILABLE,
                                Json(error),
                            ).into_response();
                        }
                    };

                    proxy_request(req, &upstream, &prefix)
                        .await
                        .map_err(|err| {
                            error!(%prefix, %upstream, %err, "proxy error");
                            StatusCode::BAD_GATEWAY
                        })
                        .into_response()
                }
            }),
        );
    }

    // Add JWT config to extensions if available
    if let Some(jwt_cfg) = jwt_config {
        router = router.layer(axum::middleware::from_fn(
            move |mut req: Request, next: Next| {
                let jwt_cfg = jwt_cfg.clone();
                async move {
                    req.extensions_mut().insert(jwt_cfg);
                    next.run(req).await
                }
            },
        ));
    }

    router
}
