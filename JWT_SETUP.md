# JWT Authentication Setup Guide

## Overview

The API Gateway now includes JWT authentication middleware to protect selected routes. By default:
- **Protected routes**: `/users/*` - Requires valid JWT token
- **Public routes**: `/orders/*` - No authentication required

## Configuration

JWT settings are configured in `config/config.toml`:

```toml
[jwt]
secret = "your-super-secret-jwt-key-change-this-in-production"
token_expiry_hours = 24
```

## Generating Test Tokens

### Option 1: Using the Helper Function (Rust)

Add this code to your `src/main.rs` or any test file:

```rust
use rust_api_gateway::middleware::generate_test_token;

fn main() {
    let secret = "your-super-secret-jwt-key-change-this-in-production";
    let token = generate_test_token("user123", 24, secret)
        .expect("Failed to generate token");
    
    println!("Generated JWT Token: {}", token);
}
```

### Option 2: Using curl with a Pre-Generated Token

```bash
# Generate token programmatically and store in a variable
TOKEN=$(cargo run --example generate_token 2>/dev/null | tail -1)

# Use the token to make a request
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/users/profile
```

### Option 3: Manual Token Generation (Using External Tools)

You can use online JWT tools like [jwt.io](https://jwt.io) or generate tokens in other languages:

**Python example:**
```python
import jwt
from datetime import datetime, timedelta

secret = "your-super-secret-jwt-key-change-this-in-production"
payload = {
    "sub": "user123",
    "exp": datetime.utcnow() + timedelta(hours=24)
}
token = jwt.encode(payload, secret, algorithm="HS256")
print(token)
```

**Node.js example:**
```javascript
const jwt = require('jsonwebtoken');

const secret = 'your-super-secret-jwt-key-change-this-in-production';
const token = jwt.sign(
    { sub: 'user123' },
    secret,
    { expiresIn: '24h', algorithm: 'HS256' }
);
console.log(token);
```

## Testing Protected Routes

### Request without token (should fail with 401):
```bash
curl http://localhost:8080/users/profile
# Response: 401 Unauthorized
```

### Request with valid token (should succeed):
```bash
TOKEN="your-generated-jwt-token-here"
curl -H "Authorization: Bearer $TOKEN" http://localhost:8080/users/profile
# Response: Proxied to upstream service
```

### Request to public route (should work without token):
```bash
curl http://localhost:8080/orders/list
# Response: Proxied to upstream service (no auth required)
```

## JWT Claims

Current JWT token structure:

```json
{
  "sub": "user123",        // Subject (typically user ID)
  "exp": 1234567890,       // Expiration time (Unix timestamp)
  "iat": 1234567800        // Issued at time (added by jsonwebtoken library)
}
```

### Extending Claims

To add more claims (roles, permissions, etc.), modify the `Claims` struct in `src/middleware/auth.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,      // Add custom claim
    pub permissions: Vec<String>, // Add custom claim
}
```

## Adding Protected Routes

To protect new routes, modify the `is_protected_route` function in `src/server/mod.rs`:

```rust
fn is_protected_route(path: &str) -> bool {
    path.starts_with("/users")
        || path.starts_with("/admin")
        || path.starts_with("/api/v1/private")
}
```

## Accessing Claims in Handlers

If you need to access claims in your upstream service, the middleware stores them in Axum's request extensions. You can extract them with:

```rust
use rust_api_gateway::middleware::Claims;
use axum::Extension;

async fn handler(Extension(claims): Extension<Claims>) {
    println!("User: {}", claims.sub);
}
```

## Production Deployment

Before going to production:

1. **Change the JWT secret** - Use a strong, random key:
   ```bash
   # Generate a strong secret
   openssl rand -base64 32
   ```

2. **Use environment variables** - Instead of hardcoding the secret in config:
   ```bash
   export JWT_SECRET="your-production-secret-key"
   export JWT_EXPIRY_HOURS="24"
   ```
   
   Then update `src/config/loader.rs` to read from environment variables.

3. **Use HTTPS** - Always use HTTPS in production to protect tokens in transit.

4. **Token Rotation** - Consider implementing token refresh mechanisms for long-lived sessions.

5. **Rate Limiting** - Add rate limiting to authentication endpoints to prevent brute-force attacks.

## Troubleshooting

### "Token validation failed" Error
- Check that the secret matches between token generation and validation
- Verify token hasn't expired
- Ensure Authorization header format is correct: `Authorization: Bearer <token>`

### Middleware not applying
- Check `is_protected_route` function includes your route path
- Verify JWT config is present in `config/config.toml`
- Check logs for initialization errors

### Tokens valid but still getting 401
- Verify the token signature is valid
- Check token expiration time with `jwt.io` decoder
- Ensure the Authorization header is spelled correctly (case-sensitive in HTTP spec)

## Next Steps

Consider implementing:
- Token refresh endpoints
- Role-Based Access Control (RBAC)
- OAuth2 integration
- Session management
- Audit logging for authentication events
