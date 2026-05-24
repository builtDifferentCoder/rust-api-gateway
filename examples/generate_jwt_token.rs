/// Example: Generate a JWT token for testing
/// 
/// Run with: cargo run --example generate_jwt_token -- [USER_ID] [SECRET] [HOURS]
/// 
/// Examples:
///   cargo run --example generate_jwt_token
///   cargo run --example generate_jwt_token -- user123
///   cargo run --example generate_jwt_token -- john_doe "my-secret-key" 48

use rust_api_gateway::middleware::generate_test_token;
use std::env;

fn main() {
    // Parse command line arguments
    let args: Vec<String> = env::args().collect();

    let user_id = args.get(1).map(|s| s.as_str()).unwrap_or("user123");
    let secret = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("your-super-secret-jwt-key-change-this-in-production");
    let expiry_hours: u32 = args
        .get(3)
        .and_then(|s| s.parse().ok())
        .unwrap_or(24);

    // Generate token
    match generate_test_token(user_id, expiry_hours, secret) {
        Ok(token) => {
            println!("✓ JWT Token Generated Successfully");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("User ID:      {}", user_id);
            println!("Expiry (hrs): {}", expiry_hours);
            println!("Token:");
            println!("{}", token);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!();
            println!("Use this token in your requests:");
            let token_preview = token[..50.min(token.len())].to_string() + "...";
            println!("  curl -H \"Authorization: Bearer {}\" http://localhost:8080/users/profile",
                     token_preview
            );
        }
        Err(e) => {
            eprintln!("✗ Failed to generate token: {}", e);
            std::process::exit(1);
        }
    }
}
