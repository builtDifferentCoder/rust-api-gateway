use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadGateway(String),
    ServiceUnavailable(String),
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        match self {
            AppError::BadGateway(msg) => {
                let body = json!({ "error": "bad_gateway", "message": msg });
                (StatusCode::BAD_GATEWAY, axum::Json(body)).into_response()
            }
            AppError::ServiceUnavailable(msg) => {
                let body = json!({ "error": "service_unavailable", "message": msg });
                (StatusCode::SERVICE_UNAVAILABLE, axum::Json(body)).into_response()
            }
            AppError::Internal(msg) => {
                let body = json!({ "error": "internal_error", "message": msg });
                (StatusCode::INTERNAL_SERVER_ERROR, axum::Json(body)).into_response()
            }
        }
    }
}

impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Internal(s)
    }
}

impl From<&str> for AppError {
    fn from(s: &str) -> Self {
        AppError::Internal(s.to_string())
    }
}
