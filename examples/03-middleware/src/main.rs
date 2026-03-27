mod auth;
mod greeter;

use gearbox_rs::{Error, Gearbox};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Error> {
    Gearbox::crank()
        .await?
        .router_with(|router| {
            router
                .layer(axum::middleware::from_fn(auth::require_auth))
                .layer(CorsLayer::permissive())
                .layer(TraceLayer::new_for_http())
        })
        .ignite()
        .await
}
