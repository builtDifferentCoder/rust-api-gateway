# JWT Authentication Implementation - Complete Reference

## Implementation Summary

JWT authentication middleware has been successfully integrated into your Rust API Gateway. This document provides a complete reference for the implementation.

---

## Files Created & Modified

### 1. **Cargo.toml** (Modified)
**Purpose**: Added JWT and cryptography dependencies

**Changes**:
- Added `jsonwebtoken = "9.2"` - JWT token creation and validation
- Added `chrono = { version = "0.4", features = ["serde"] }` - Timestamp handling
- Added `serde_json = "1.0"` - JSON serialization

### 2. **src/middleware/auth.rs** (Created)
**Purpose**: Core JWT authentication middleware implementation

**Key Components**:
- `Claims` struct - JWT payload containing `sub` (subject) and `exp` (expiration)
- `JwtConfig` struct - Configuration for JWT validation
- `extract_token()` - Extracts Bearer token from Authorization header
- `validate_token()` - Validates JWT signature and expiry
- `jwt_auth_middleware()` - Main middleware that validates tokens and stores claims in request extensions
- `generate_test_token()` - Helper function for generating test tokens
- Unit tests for all functionality

**Middleware Flow**:
```
Request
  ↓
Extract Authorization header
  ↓
Parse Bearer token
  ↓
Validate JWT signature and expiry
  ↓
Store Claims in request extensions
  ↓
Pass to next middleware/handler
  ↓
On error: Return 401 Unauthorized
```

### 3. **src/middleware/mod.rs** (Modified)
**Purpose**: Export auth module and public types

**Exports**:
- `jwt_auth_middleware` - The authentication middleware
- `JwtConfig` - Configuration struct
- `Claims` - Token claims struct
- `generate_test_token` - Token generation helper

### 4. **src/config/loader.rs** (Modified)
**Purpose**: Add JWT configuration loading from TOML

**New Structs**:
- `JwtConfigFile` - Maps TOML JWT configuration
- Modified `Config` struct to include optional JWT config

**Configuration Example**:
```toml
[jwt]
secret = "your-super-secret-jwt-key-change-this-in-production"
token_expiry_hours = 24
```

### 5. **config/config.toml** (Modified)
**Purpose**: Configuration file with JWT settings

**Added**:
```toml
[jwt]
secret = "your-super-secret-jwt-key-change-this-in-production"
token_expiry_hours = 24
```

### 6. **src/router/router.rs** (Modified)
**Purpose**: Add JWT config to request extensions

**Changes**:
- Create JWT config from application configuration
- Store JWT config in Axum request extensions
- Configuration available to middleware layer

### 7. **src/server/mod.rs** (Modified)
**Purpose**: Apply conditional JWT middleware

**New Function**:
- `conditional_jwt_middleware()` - Applies JWT validation only to protected routes

**New Function**:
- `is_protected_route()` - Determines which routes require authentication

**Middleware Stack** (execution order):
1. Conditional JWT middleware (protects /users/*)
2. Request logging middleware
3. Router

### 8. **examples/generate_jwt_token.rs** (Created)
**Purpose**: Utility to generate test JWT tokens

**Usage**:
```bash
cargo run --example generate_jwt_token
cargo run --example generate_jwt_token -- user123
cargo run --example generate_jwt_token -- john_doe "my-secret" 48
```

**Output**: Displays generated token and example curl command

### 9. **JWT_SETUP.md** (Created)
**Purpose**: Complete JWT authentication guide and documentation

---

## Architecture Overview

### Request Flow for Protected Route (/users/*)

```
Client Request with Token
         ↓
   HTTP Server
         ↓
Request Logger (tracing)
         ↓
Conditional JWT Middleware
         ↓
    ┌─────────────────┐
    │ Is Protected?   │
    └────────┬────────┘
         YES │
             ↓
    ┌──────────────────────┐
    │ Extract Bearer Token │ ← Authorization: Bearer <token>
    └────────┬─────────────┘
             ↓
    ┌──────────────────┐
    │ Validate Signature│ ← Uses JWT secret from config
    │ Check Expiration │
    └────────┬─────────┘
         PASS│
             ↓
    ┌──────────────────┐
    │ Store Claims in  │ ← Accessible via request.extensions()
    │ Request Ext.     │
    └────────┬─────────┘
             ↓
         Router
             ↓
    Reverse Proxy Handler
             ↓
       Upstream Service
```

### Request Flow for Public Route (/orders/*)

```
Client Request
         ↓
   HTTP Server
         ↓
Request Logger (tracing)
         ↓
Conditional JWT Middleware
         ↓
    ┌─────────────────┐
    │ Is Protected?   │
    └────────┬────────┘
         NO  │ (Skip JWT validation)
             ↓
         Router
             ↓
    Reverse Proxy Handler
             ↓
       Upstream Service
```

---

## Route Protection Matrix

| Route | Protected | Requires JWT | Example |
|-------|-----------|-------------|---------|
| `/users/*` | ✓ | ✓ | `/users/profile` |
| `/orders/*` | ✗ | ✗ | `/orders/list` |
| `/{other}/*` | ✗ | ✗ | `/health/status` |

---

## JWT Token Structure

### Header
```json
{
  "alg": "HS256",
  "typ": "JWT"
}
```

### Payload (Claims)
```json
{
  "sub": "user123",      // Subject (user ID)
  "exp": 1779652598,     // Expiration time (Unix timestamp)
  "iat": 1779566198      // Issued at time (auto-generated)
}
```

### Signature
```
HMACSHA256(
  base64UrlEncode(header) + "." + base64UrlEncode(payload),
  secret
)
```

---

## Testing the Implementation

### 1. Generate a Test Token
```bash
cargo run --example generate_jwt_token -- myuser
```

### 2. Test Protected Route (Should FAIL without token)
```bash
curl -i http://localhost:8080/users/profile
# Response: 401 Unauthorized
```

### 3. Test Protected Route (Should SUCCEED with token)
```bash
TOKEN="<generated-token>"
curl -i -H "Authorization: Bearer $TOKEN" http://localhost:8080/users/profile
# Response: 200 OK (proxied to upstream)
```

### 4. Test Public Route (Should SUCCEED without token)
```bash
curl -i http://localhost:8080/orders/list
# Response: 200 OK (proxied to upstream)
```

---

## Configuration Reference

### Minimal Configuration
```toml
[[routes]]
path = "/users"
upstreams = ["http://localhost:3001"]

[jwt]
secret = "your-secret-key"
```

### Complete Configuration
```toml
host = "0.0.0.0"
port = 8080

[[routes]]
path = "/users"
upstreams = ["http://localhost:3001", "http://localhost:3003"]

[[routes]]
path = "/orders"
upstreams = ["http://localhost:3002"]

[jwt]
secret = "your-super-secret-jwt-key"
token_expiry_hours = 24
```

---

## Adding More Protected Routes

Edit the `is_protected_route()` function in `src/server/mod.rs`:

```rust
fn is_protected_route(path: &str) -> bool {
    path.starts_with("/users")
        || path.starts_with("/admin")
        || path.starts_with("/api/v1/private")
        || path.starts_with("/sensitive-data")
}
```

---

## Extending Claims

To add custom claims (e.g., roles, permissions):

1. **Update Claims struct** in `src/middleware/auth.rs`:
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub role: String,      // NEW
    pub permissions: Vec<String>, // NEW
}
```

2. **Update token generation** in `generate_test_token()`:
```rust
let claims = Claims {
    sub: subject.to_string(),
    exp: expiry.timestamp() as usize,
    role: "user".to_string(),
    permissions: vec!["read".to_string()],
};
```

3. **Use claims in handlers**:
```rust
use axum::Extension;
use rust_api_gateway::middleware::Claims;

async fn my_handler(Extension(claims): Extension<Claims>) {
    println!("User {} with role {}", claims.sub, claims.role);
}
```

---

## Error Responses

| Status | Scenario | Response Header |
|--------|----------|-----------------|
| 401 | Missing Authorization header | `WWW-Authenticate: Bearer` |
| 401 | Invalid token format | `WWW-Authenticate: Bearer` |
| 401 | Invalid signature | `WWW-Authenticate: Bearer` |
| 401 | Token expired | `WWW-Authenticate: Bearer` |
| 500 | Internal error (missing JWT config) | N/A |

---

## Performance Considerations

- **Token validation is fast** - Direct signature verification, no database lookup
- **No caching required** - Validation happens per-request (milliseconds)
- **Stateless** - No session storage needed
- **Scalable** - Works across multiple server instances with same secret

---

## Security Checklist for Production

- [ ] Change JWT secret from default value
- [ ] Use strong, random secret (minimum 32 characters)
- [ ] Enable HTTPS on all routes
- [ ] Set appropriate token expiry time
- [ ] Validate tokens on every protected route
- [ ] Log authentication failures
- [ ] Consider rate limiting for token generation
- [ ] Regularly rotate secrets (implement token refresh)
- [ ] Keep dependencies updated (`cargo update`)

---

## Next Steps for Enhancement

1. **Token Refresh**: Implement short-lived tokens + refresh tokens
2. **RBAC**: Add role-based access control to middleware
3. **Audit Logging**: Log all authentication attempts
4. **Token Blacklist**: Implement token revocation
5. **Rate Limiting**: Protect against brute-force attacks
6. **OAuth2**: Integrate with external identity providers
7. **Custom Claims**: Add more information to JWT payload

---

## Troubleshooting

### Issue: 401 Unauthorized on all requests
**Solution**: 
- Verify JWT secret matches between config and token generation
- Check Authorization header format: `Bearer <token>`
- Verify token hasn't expired using jwt.io

### Issue: Middleware not enforcing authentication
**Solution**:
- Check `is_protected_route()` includes your route
- Verify JWT config is present in config.toml
- Check logs for initialization errors

### Issue: Token validation fails
**Solution**:
- Ensure secret in config.toml matches token secret
- Verify token hasn't expired
- Check token wasn't tampered with

---

## References

- [jsonwebtoken crate docs](https://docs.rs/jsonwebtoken/)
- [JWT introduction](https://jwt.io/introduction)
- [Axum middleware guide](https://docs.rs/axum/latest/axum/middleware/)
- [OWASP JWT guide](https://cheatsheetseries.owasp.org/cheatsheets/JSON_Web_Token_for_Java_Cheat_Sheet.html)
