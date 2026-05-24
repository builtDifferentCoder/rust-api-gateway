use serde::Deserialize;
use std::env;
use std::fs;
use std::path::Path;
use tracing::warn;

#[derive(Debug, Deserialize, Clone)]
pub struct RouteConfig {
    pub path: String,
    pub upstreams: Vec<String>,
    #[serde(default = "default_health_path")]
    pub health_path: String,
}

fn default_health_path() -> String {
    "/health".to_string()
}

#[derive(Debug, Deserialize, Clone)]
pub struct JwtConfigFile {
    pub secret: String,
    #[serde(default = "default_token_expiry_hours")]
    pub token_expiry_hours: u32,
}

fn default_token_expiry_hours() -> u32 {
    24
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfigFile {
    #[serde(default = "default_requests_per_minute")]
    pub requests_per_minute: u32,
}

fn default_requests_per_minute() -> u32 {
    60
}

#[derive(Debug, Deserialize, Clone)]
pub struct HealthConfigFile {
    #[serde(default = "default_health_interval")]
    pub interval_seconds: u64,
}

fn default_health_interval() -> u64 {
    10
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub routes: Vec<RouteConfig>,
    #[serde(default)]
    pub jwt: Option<JwtConfigFile>,
    #[serde(default)]
    pub rate_limit: Option<RateLimitConfigFile>,
    #[serde(default)]
    pub health: Option<HealthConfigFile>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".to_string(),
            port: 8080,
            routes: Vec::new(),
            jwt: None,
            rate_limit: None,
            health: None,
        }
    }
}

#[derive(Debug, Deserialize)]
struct FileConfig {
    host: Option<String>,
    port: Option<u16>,
    routes: Option<Vec<RouteConfig>>,
    jwt: Option<JwtConfigFile>,
    rate_limit: Option<RateLimitConfigFile>,
    health: Option<HealthConfigFile>,
}

pub fn load_config() -> Config {
    let default = Config::default();
    let path = Path::new("config/config.toml");

    if !path.exists() {
        return default;
    }

    let contents = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) => {
            warn!("Failed to read config/config.toml, using defaults: {}", err);
            return default;
        }
    };
    let mut config = match toml::from_str::<FileConfig>(&contents) {
        Ok(parsed) => Config {
            host: parsed.host.unwrap_or(default.host),
            port: parsed.port.unwrap_or(default.port),
            routes: parsed.routes.unwrap_or(default.routes),
            jwt: parsed.jwt,
            rate_limit: parsed.rate_limit,
            health: parsed.health,
        },
        Err(err) => {
            warn!("Failed to parse config/config.toml, using defaults: {}", err);
            default
        }
    };

    // Environment overrides
    if let Ok(host) = env::var("HOST") {
        config.host = host;
    }

    if let Ok(port_str) = env::var("PORT") {
        if let Ok(port) = port_str.parse::<u16>() {
            config.port = port;
        } else {
            warn!("Invalid PORT env var: {}", port_str);
        }
    }

    if let Ok(rpm_str) = env::var("RATE_LIMIT_REQUESTS_PER_MINUTE") {
        if let Ok(rpm) = rpm_str.parse::<u32>() {
            config.rate_limit = Some(RateLimitConfigFile { requests_per_minute: rpm });
        } else {
            warn!("Invalid RATE_LIMIT_REQUESTS_PER_MINUTE: {}", rpm_str);
        }
    }

    if let Ok(hint_str) = env::var("HEALTH_INTERVAL_SECONDS") {
        if let Ok(iv) = hint_str.parse::<u64>() {
            config.health = Some(HealthConfigFile { interval_seconds: iv });
        } else {
            warn!("Invalid HEALTH_INTERVAL_SECONDS: {}", hint_str);
        }
    }

    config
}
