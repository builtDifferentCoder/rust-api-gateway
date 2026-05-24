use axum::{
    extract::ConnectInfo,
    http::{StatusCode},
    middleware::Next,
    response::{IntoResponse},
    Json,
};
use serde_json::json;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for rate limiting
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Number of requests allowed per minute
    pub requests_per_minute: u32,
    /// Time window for rate limiting (in seconds)
    pub window_seconds: u64,
}

impl RateLimitConfig {
    /// Create a new rate limit configuration
    pub fn new(requests_per_minute: u32) -> Self {
        Self {
            requests_per_minute,
            window_seconds: 60,
        }
    }
}

/// Represents a client's token bucket state
#[derive(Debug, Clone)]
struct TokenBucket {
    /// Number of tokens available
    tokens: f64,
    /// Last time the bucket was updated
    last_updated: Instant,
}

impl TokenBucket {
    /// Create a new token bucket with full capacity
    fn new(capacity: f64) -> Self {
        Self {
            tokens: capacity,
            last_updated: Instant::now(),
        }
    }

    /// Update the bucket: add tokens based on elapsed time
    fn refill(&mut self, capacity: f64, refill_rate: f64) {
        let elapsed = self.last_updated.elapsed().as_secs_f64();
        let tokens_to_add = elapsed * refill_rate;
        
        self.tokens = (self.tokens + tokens_to_add).min(capacity);
        self.last_updated = Instant::now();
    }

    /// Check if we can consume the required tokens
    fn try_consume(&mut self, tokens_needed: f64) -> bool {
        if self.tokens >= tokens_needed {
            self.tokens -= tokens_needed;
            true
        } else {
            false
        }
    }

    /// Get remaining tokens
    fn remaining_tokens(&self) -> f64 {
        self.tokens
    }
}

/// Rate limiter using token bucket algorithm
pub struct RateLimiter {
    /// Configuration for rate limiting
    config: RateLimitConfig,
    /// Token buckets per IP address
    buckets: Arc<Mutex<HashMap<String, TokenBucket>>>,
    /// Capacity of each bucket (full amount)
    capacity: f64,
    /// Refill rate (tokens per second)
    refill_rate: f64,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        let capacity = config.requests_per_minute as f64;
        let refill_rate = capacity / (config.window_seconds as f64);

        Self {
            config,
            buckets: Arc::new(Mutex::new(HashMap::new())),
            capacity,
            refill_rate,
        }
    }

    /// Check if a request from the given IP is allowed
    pub fn is_allowed(&self, ip: &str) -> (bool, u32) {
        let mut buckets = self.buckets.lock().unwrap();
        
        let bucket = buckets
            .entry(ip.to_string())
            .or_insert_with(|| TokenBucket::new(self.capacity));

        // Refill tokens based on time passed
        bucket.refill(self.capacity, self.refill_rate);

        // Try to consume 1 token
        let allowed = bucket.try_consume(1.0);
        let remaining = bucket.remaining_tokens().ceil() as u32;

        (allowed, remaining)
    }

    /// Get current request count for an IP (for testing/monitoring)
    pub fn get_remaining_requests(&self, ip: &str) -> Option<u32> {
        let buckets = self.buckets.lock().unwrap();
        buckets.get(ip).map(|bucket| bucket.remaining_tokens().ceil() as u32)
    }

    /// Clean up old entries (optional, for memory management)
    /// This should be called periodically to remove stale IPs
    #[allow(dead_code)]
    pub fn cleanup_stale_entries(&self, _threshold: Duration) {
        // For now, we'll keep it simple and not clean up
        // In production, you'd want to remove entries that haven't been accessed
        // This prevents unbounded memory growth from tracking every IP
    }
}

impl Clone for RateLimiter {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            buckets: self.buckets.clone(),
            capacity: self.capacity,
            refill_rate: self.refill_rate,
        }
    }
}

/// JSON error response for rate limiting
#[derive(serde::Serialize)]
struct RateLimitError {
    error: String,
    message: String,
    retry_after: u32,
}

/// Extract client IP from request
fn get_client_ip(req: &axum::extract::Request) -> String {
    // Try to get from ConnectInfo (most reliable)
    if let Some(ConnectInfo(addr)) = req.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }

    // Fallback to X-Forwarded-For header (for proxied requests)
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            // X-Forwarded-For can contain multiple IPs, take the first one
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    // Fallback to X-Real-IP header
    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(real_ip_str) = real_ip.to_str() {
            return real_ip_str.to_string();
        }
    }

    // Default to unknown
    "unknown".to_string()
}

/// Rate limiting middleware
/// 
/// This middleware:
/// 1. Extracts client IP from request
/// 2. Checks token bucket for that IP
/// 3. Allows request if tokens available
/// 4. Returns 429 Too Many Requests if limit exceeded
pub async fn rate_limit_middleware(
    req: axum::extract::Request,
    next: Next,
) -> Result<axum::response::Response, StatusCode> {
    // Get rate limiter from extensions (set by the app layer)
    let rate_limiter = req
        .extensions()
        .get::<RateLimiter>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract client IP
    let client_ip = get_client_ip(&req);

    // Check if request is allowed
    let (allowed, remaining) = rate_limiter.is_allowed(&client_ip);

    if allowed {
        // Request is allowed, continue
        let mut res = next.run(req).await;
        
        // Add rate limit headers to response
        res.headers_mut().insert(
            "X-RateLimit-Limit",
            rate_limiter.config.requests_per_minute.to_string().parse().unwrap(),
        );
        res.headers_mut().insert(
            "X-RateLimit-Remaining",
            remaining.to_string().parse().unwrap(),
        );
        
        Ok(res)
    } else {
        // Rate limit exceeded
        Err(StatusCode::TOO_MANY_REQUESTS)
    }
}

/// Custom error response for rate limiting
pub async fn rate_limit_error_response() -> impl IntoResponse {
    let error = RateLimitError {
        error: "rate_limit_exceeded".to_string(),
        message: "Too many requests. Please try again later.".to_string(),
        retry_after: 60,
    };

    (
        StatusCode::TOO_MANY_REQUESTS,
        [
            ("Content-Type", "application/json"),
            ("Retry-After", "60"),
        ],
        Json(error),
    )
}

/// Alternative rate limiting middleware that returns proper error response
pub async fn rate_limit_middleware_v2(
    req: axum::extract::Request,
    next: Next,
) -> axum::response::Response {
    // Get rate limiter from extensions (set by the app layer)
    let rate_limiter = match req.extensions().get::<RateLimiter>().cloned() {
        Some(rl) => rl,
        None => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "Rate limiter not configured" })),
            )
                .into_response();
        }
    };

    // Extract client IP
    let client_ip = get_client_ip(&req);

    // Check if request is allowed
    let (allowed, remaining) = rate_limiter.is_allowed(&client_ip);

    if allowed {
        // Request is allowed, continue
        let mut res = next.run(req).await;
        
        // Add rate limit headers to response
        let _ = res.headers_mut().insert(
            "X-RateLimit-Limit",
            rate_limiter.config.requests_per_minute.to_string().parse().unwrap(),
        );
        let _ = res.headers_mut().insert(
            "X-RateLimit-Remaining",
            remaining.to_string().parse().unwrap(),
        );
        
        res
    } else {
        // Rate limit exceeded
        let error = json!({
            "error": "rate_limit_exceeded",
            "message": "Too many requests. Please try again later.",
            "retry_after": 60
        });

        (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Content-Type", "application/json"),
                ("Retry-After", "60"),
            ],
            Json(error),
        )
            .into_response()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket_creation() {
        let bucket = TokenBucket::new(60.0);
        assert_eq!(bucket.remaining_tokens() as u32, 60);
    }

    #[test]
    fn test_token_consumption() {
        let mut bucket = TokenBucket::new(60.0);
        assert!(bucket.try_consume(1.0));
        assert_eq!(bucket.remaining_tokens() as u32, 59);
    }

    #[test]
    fn test_token_consumption_exceeds_available() {
        let mut bucket = TokenBucket::new(60.0);
        assert!(bucket.try_consume(60.0));
        assert!(!bucket.try_consume(1.0));
    }

    #[test]
    fn test_rate_limiter_allows_requests_within_limit() {
        let config = RateLimitConfig::new(60);
        let limiter = RateLimiter::new(config);

        let ip = "192.168.1.1";

        // Should allow 60 requests
        for _ in 0..60 {
            let (allowed, _) = limiter.is_allowed(ip);
            assert!(allowed);
        }

        // 61st request should be denied
        let (allowed, _) = limiter.is_allowed(ip);
        assert!(!allowed);
    }

    #[test]
    fn test_rate_limiter_different_ips() {
        let config = RateLimitConfig::new(5);
        let limiter = RateLimiter::new(config);

        let ip1 = "192.168.1.1";
        let ip2 = "192.168.1.2";

        // Each IP should have its own limit
        for _ in 0..5 {
            let (allowed1, _) = limiter.is_allowed(ip1);
            let (allowed2, _) = limiter.is_allowed(ip2);
            assert!(allowed1);
            assert!(allowed2);
        }

        // Both should be rate limited now
        let (allowed1, _) = limiter.is_allowed(ip1);
        let (allowed2, _) = limiter.is_allowed(ip2);
        assert!(!allowed1);
        assert!(!allowed2);
    }

    #[test]
    fn test_rate_limiter_config() {
        let config = RateLimitConfig::new(100);
        assert_eq!(config.requests_per_minute, 100);
        assert_eq!(config.window_seconds, 60);
    }
}
