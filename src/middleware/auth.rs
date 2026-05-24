use axum::{
    extract::Request,
    http::{header::HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// JWT Claims structure containing user information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    /// Subject (typically user ID)
    pub sub: String,
    /// Expiration time (Unix timestamp)
    pub exp: usize,
}

/// JWT authentication configuration
#[derive(Clone)]
pub struct JwtConfig {
    /// Secret key for JWT validation
    pub secret: String,
    /// Token expiry duration in hours
    pub token_expiry_hours: u32,
}

impl JwtConfig {
    /// Create a new JWT configuration
    pub fn new(secret: String, token_expiry_hours: u32) -> Self {
        Self {
            secret,
            token_expiry_hours,
        }
    }

    /// Get the decoding key for JWT validation
    fn get_decoding_key(&self) -> DecodingKey {
        DecodingKey::from_secret(self.secret.as_bytes())
    }
}

/// Extract Bearer token from Authorization header
fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| {
            if s.starts_with("Bearer ") {
                Some(s[7..].to_string())
            } else {
                None
            }
        })
}

/// Validate JWT token and extract claims
pub fn validate_token(token: &str, config: &JwtConfig) -> Result<Claims, String> {
    let decoding_key = config.get_decoding_key();
    let validation = Validation::default();

    decode::<Claims>(token, &decoding_key, &validation)
        .map(|data| data.claims)
        .map_err(|err| format!("Token validation failed: {}", err))
}

/// JWT authentication middleware
/// 
/// This middleware:
/// 1. Extracts the Bearer token from the Authorization header
/// 2. Validates the JWT token signature and expiry
/// 3. Stores validated claims in request extensions
/// 4. Rejects invalid/missing tokens with 401 Unauthorized
pub async fn jwt_auth_middleware(
    headers: HeaderMap,
    mut req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    // Extract JWT config from request extensions (set by the app layer)
    let config = req
        .extensions()
        .get::<Arc<JwtConfig>>()
        .cloned()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Extract token from Authorization header
    let token = extract_token(&headers).ok_or(StatusCode::UNAUTHORIZED)?;

    // Validate token
    let claims = validate_token(&token, &config).map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Store claims in request extensions for downstream handlers
    req.extensions_mut().insert(claims);

    // Continue to next middleware/handler
    Ok(next.run(req).await)
}

/// Helper function to generate a test JWT token
/// 
/// **WARNING: This is for testing only. Do NOT use in production.**
/// 
/// Example:
/// ```ignore
/// let token = generate_test_token("user123", 24, "your-super-secret-jwt-key-change-this-in-production");
/// ```
pub fn generate_test_token(
    subject: &str,
    expiry_hours: u32,
    secret: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    use chrono::Utc;
    use jsonwebtoken::{encode, EncodingKey, Header};

    let now = Utc::now();
    let expiry = now + chrono::Duration::hours(expiry_hours as i64);

    let claims = Claims {
        sub: subject.to_string(),
        exp: expiry.timestamp() as usize,
    };

    let encoding_key = EncodingKey::from_secret(secret.as_bytes());
    encode(&Header::default(), &claims, &encoding_key)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_and_validate_token() {
        let secret = "test-secret-key";
        let subject = "user123";
        let expiry_hours = 24;

        // Generate token
        let token = generate_test_token(subject, expiry_hours, secret)
            .expect("Failed to generate token");

        // Validate token
        let config = JwtConfig::new(secret.to_string(), expiry_hours);
        let claims = validate_token(&token, &config).expect("Failed to validate token");

        assert_eq!(claims.sub, subject);
    }

    #[test]
    fn test_invalid_token_rejected() {
        let config = JwtConfig::new("secret".to_string(), 24);
        let result = validate_token("invalid.token.here", &config);
        assert!(result.is_err());
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "authorization",
            "Bearer test-token-value".parse().unwrap(),
        );

        let token = extract_token(&headers);
        assert_eq!(token, Some("test-token-value".to_string()));
    }

    #[test]
    fn test_missing_bearer_prefix() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "test-token-value".parse().unwrap());

        let token = extract_token(&headers);
        assert_eq!(token, None);
    }
}
