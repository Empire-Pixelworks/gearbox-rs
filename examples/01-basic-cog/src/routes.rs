use crate::service::{CreateUser, UserService};
use axum::Json;
use axum::extract::Path;
use gearbox_rs::{IntoResponse, get, post};

#[get("/users")]
async fn list_users(service: Arc<UserService>) -> impl IntoResponse {
    Json(service.list_users())
}

#[get("/users/{id}")]
async fn get_user(service: Arc<UserService>, Path(id): Path<String>) -> impl IntoResponse {
    match service.get_user(&id) {
        Some(user) => Json(user).into_response(),
        None => (axum::http::StatusCode::NOT_FOUND, "User not found").into_response(),
    }
}

#[post("/users")]
async fn create_user(
    service: Arc<UserService>,
    Json(input): Json<CreateUser>,
) -> impl IntoResponse {
    let user = service.create_user(input);
    (axum::http::StatusCode::CREATED, Json(user))
}
