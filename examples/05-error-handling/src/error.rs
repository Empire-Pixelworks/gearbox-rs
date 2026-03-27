use axum::Json;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::Serialize;
use thiserror::Error;

/// Application-level error type that maps to HTTP status codes.
///
/// Implement `IntoResponse` so route handlers can return `Result<T, AppError>`
/// and axum will convert errors into proper HTTP responses automatically.
#[derive(Debug, Error)]
pub enum AppError {
    #[error("{0}")]
    NotFound(String),

    #[error("{0}")]
    BadRequest(String),

    #[error("internal error: {0}")]
    Internal(String),
}

#[derive(Serialize)]
struct ErrorBody {
    error: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::BadRequest(_) => StatusCode::BAD_REQUEST,
            AppError::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = ErrorBody {
            error: self.to_string(),
        };

        (status, Json(body)).into_response()
    }
}

/// Convert framework errors into AppError so `?` works in handlers.
impl From<gearbox_rs::Error> for AppError {
    fn from(err: gearbox_rs::Error) -> Self {
        AppError::Internal(err.to_string())
    }
}
