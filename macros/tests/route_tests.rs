//! Integration tests for the route macros (#[get], #[post], etc.)

use gearbox_rs_core::{IntoResponse, Json, Query, RouteRegistration};
use gearbox_rs_macros::{cog, delete, get, patch, post, put};

// Arc is used in handler signatures but transformed to Inject by the macros
#[allow(unused_imports)]
use std::sync::Arc;

// === Test Cog for injection tests ===

#[cog]
pub struct TestService {
    pub value: String,
}

impl TestService {
    pub fn get_value(&self) -> &str {
        &self.value
    }
}

// === Basic route registration tests ===

#[get("/test-get")]
async fn test_get_handler() -> impl IntoResponse {
    "get"
}

#[post("/test-post")]
async fn test_post_handler() -> impl IntoResponse {
    "post"
}

#[put("/test-put")]
async fn test_put_handler() -> impl IntoResponse {
    "put"
}

#[delete("/test-delete")]
async fn test_delete_handler() -> impl IntoResponse {
    "delete"
}

#[patch("/test-patch")]
async fn test_patch_handler() -> impl IntoResponse {
    "patch"
}

#[test]
fn test_route_registration_exists() {
    // Verify routes are registered via inventory
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();

    let paths: Vec<&str> = routes.iter().map(|r| r.path).collect();

    assert!(paths.contains(&"/test-get"), "GET route not registered");
    assert!(paths.contains(&"/test-post"), "POST route not registered");
    assert!(paths.contains(&"/test-put"), "PUT route not registered");
    assert!(paths.contains(&"/test-delete"), "DELETE route not registered");
    assert!(paths.contains(&"/test-patch"), "PATCH route not registered");
}

#[test]
fn test_route_methods_correct() {
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();

    for route in routes {
        match route.path {
            "/test-get" => assert_eq!(route.method, "GET"),
            "/test-post" => assert_eq!(route.method, "POST"),
            "/test-put" => assert_eq!(route.method, "PUT"),
            "/test-delete" => assert_eq!(route.method, "DELETE"),
            "/test-patch" => assert_eq!(route.method, "PATCH"),
            _ => {} // Other routes from other tests
        }
    }
}

// === Arc parameter transformation tests ===
// These test that Arc<T> params are transformed to Inject<T> extractors

#[get("/with-arc")]
async fn handler_with_arc(service: Arc<TestService>) -> impl IntoResponse {
    service.get_value().to_string()
}

#[get("/with-multiple-arcs")]
async fn handler_with_multiple_arcs(
    _service1: Arc<TestService>,
    _service2: Arc<TestService>,
) -> impl IntoResponse {
    "multiple"
}

#[test]
fn test_arc_routes_registered() {
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();
    let paths: Vec<&str> = routes.iter().map(|r| r.path).collect();

    assert!(paths.contains(&"/with-arc"));
    assert!(paths.contains(&"/with-multiple-arcs"));
}

// === Mixed parameter tests ===

#[derive(serde::Deserialize)]
pub struct QueryParams {
    #[allow(dead_code)]
    pub search: Option<String>,
}

#[get("/mixed-params")]
async fn handler_mixed_params(
    _service: Arc<TestService>,
    _query: Query<QueryParams>,
) -> impl IntoResponse {
    "mixed"
}

#[post("/with-json")]
async fn handler_with_json(
    _service: Arc<TestService>,
    _body: Json<QueryParams>,
) -> impl IntoResponse {
    "json"
}

#[test]
fn test_mixed_params_routes_registered() {
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();
    let paths: Vec<&str> = routes.iter().map(|r| r.path).collect();

    assert!(paths.contains(&"/mixed-params"));
    assert!(paths.contains(&"/with-json"));
}

// === Path parameter tests ===

#[get("/users/:id")]
async fn handler_with_path_param(
    _path: gearbox_rs_core::Path<String>,
    _service: Arc<TestService>,
) -> impl IntoResponse {
    "user"
}

#[test]
fn test_path_param_route_registered() {
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();
    let paths: Vec<&str> = routes.iter().map(|r| r.path).collect();

    assert!(paths.contains(&"/users/:id"));
}

// === No parameters test ===

#[get("/health")]
async fn health_check() -> impl IntoResponse {
    "ok"
}

#[test]
fn test_no_params_route_registered() {
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();
    let paths: Vec<&str> = routes.iter().map(|r| r.path).collect();

    assert!(paths.contains(&"/health"));
}

// === Return type tests ===

#[get("/returns-json")]
async fn returns_json() -> Json<Vec<String>> {
    Json(vec!["a".to_string(), "b".to_string()])
}

#[get("/returns-result")]
async fn returns_result() -> Result<Json<String>, String> {
    Ok(Json("success".to_string()))
}

#[test]
fn test_return_type_routes_registered() {
    let routes: Vec<_> = inventory::iter::<RouteRegistration>().collect();
    let paths: Vec<&str> = routes.iter().map(|r| r.path).collect();

    assert!(paths.contains(&"/returns-json"));
    assert!(paths.contains(&"/returns-result"));
}
