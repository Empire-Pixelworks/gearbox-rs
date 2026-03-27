use crate::error::AppError;
use crate::service::{CreateItem, Item, ItemService};
use axum::Json;
use axum::extract::Path;
use axum::http::StatusCode;
use gearbox_rs::{get, post};

/// List all items. Infallible — always returns 200.
#[get("/items")]
async fn list_items(service: Arc<ItemService>) -> Json<Vec<Item>> {
    Json(service.list_items())
}

/// Get a single item by ID.
/// Returns 404 with a JSON error body if the item doesn't exist.
#[get("/items/{id}")]
async fn get_item(
    service: Arc<ItemService>,
    Path(id): Path<String>,
) -> Result<Json<Item>, AppError> {
    // The `?` operator propagates AppError::NotFound from the service layer.
    let item = service.get_item(&id)?;
    Ok(Json(item))
}

/// Create a new item.
/// Returns 400 if the name is empty, 201 on success.
#[post("/items")]
async fn create_item(
    service: Arc<ItemService>,
    Json(input): Json<CreateItem>,
) -> Result<(StatusCode, Json<Item>), AppError> {
    let item = service.create_item(input)?;
    Ok((StatusCode::CREATED, Json(item)))
}
