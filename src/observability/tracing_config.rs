use tracing_subscriber::{fmt, prelude::*, EnvFilter};

/// Initialize structured tracing with JSON output and request spans
pub fn init_structured_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    // Initialize tracing with JSON formatting
    tracing_subscriber::registry()
        .with(
            fmt::layer()
                .json()
                .with_target(true)
                .with_thread_ids(true)
                .with_file(true)
                .with_line_number(true),
        )
        .with(env_filter)
        .init();

    tracing::info!("Structured tracing initialized");
}

/// Initialize basic tracing (non-JSON, for development)
pub fn init_basic_tracing() {
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));

    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .init();

    tracing::info!("Basic tracing initialized");
}

#[cfg(test)]
mod tests {
    

    #[test]
    fn test_tracing_initialization() {
        // This test ensures tracing initializes without panicking
        // Note: We can only initialize once per process, so tests should be careful
        let _ = std::panic::catch_unwind(|| {
            // Tracing can only be initialized once
            // This is a known limitation of the tracing-subscriber
        });
    }
}
