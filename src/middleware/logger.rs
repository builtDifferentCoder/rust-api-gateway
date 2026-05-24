use tower_http::trace::TraceLayer;
use tower_http::classify::{ServerErrorsAsFailures, SharedClassifier};

/// Create a middleware layer that logs method, path, status, and duration for each request.
///
/// Uses `tower_http::trace::TraceLayer` and `tracing` for structured logging.
pub fn request_logger_layer() -> TraceLayer<SharedClassifier<ServerErrorsAsFailures>> {
    TraceLayer::new_for_http()
}
