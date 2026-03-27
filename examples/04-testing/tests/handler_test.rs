//! Demonstrates testing Axum route handlers using TestHubBuilder
//! and tower::ServiceExt::oneshot (no running server needed).

use axum::body::Body;
use axum::http::{Request, StatusCode};
use gearbox_rs::{Cog, Hub, TestHubBuilder};
use http_body_util::BodyExt;
use std::sync::Arc;
use testing_example::service::{GreetingConfig, GreetingService};
use tower::ServiceExt;

/// Build a test router with the greeting route wired up.
fn test_router(hub: Arc<Hub>) -> axum::Router {
    axum::Router::new()
        .route(
            "/greet/{name}",
            axum::routing::get(
                |name: axum::extract::Path<String>,
                 service: gearbox_rs::Inject<GreetingService>| async move {
                    service.greet(&name)
                },
            ),
        )
        .with_state(hub)
}

#[tokio::test]
async fn test_greet_handler_returns_200() {
    let hub = TestHubBuilder::new()
        .with_config(GreetingConfig {
            message: "Hey".to_string(),
        })
        .build();

    // Register the service into the hub so Inject<GreetingService> can find it
    let service = GreetingService::new(Arc::clone(&hub)).await.unwrap();
    hub.registry_put(service).unwrap();

    let app = test_router(hub);

    let response = app
        .oneshot(
            Request::builder()
                .uri("/greet/Tester")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = response.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8(body.to_vec()).unwrap();
    assert_eq!(body_str, "Hey, Tester!");
}
