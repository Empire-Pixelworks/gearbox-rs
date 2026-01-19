mod repo;
mod service;
mod routes;

use gearbox_core::{Error, Gearbox};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt::init();
    Gearbox::crank().await?.ignite().await
}
