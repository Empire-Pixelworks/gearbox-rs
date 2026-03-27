use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

pub async fn require_auth(request: Request, next: Next) -> Result<Response, StatusCode> {
    match request.headers().get("authorization") {
        Some(value) if value == "Bearer secret-token" => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
