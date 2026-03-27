use gearbox_rs::{Error, IntoResponse, Json, cog, get};

#[cog]
#[on_start(initialize)]
#[on_shutdown(cleanup)]
pub struct Greeter {}

impl Greeter {
    pub fn greet(&self, name: &str) -> String {
        format!("Hello, {}!", name)
    }

    async fn initialize(&self) -> Result<(), Error> {
        tracing::info!("Greeter started");
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), Error> {
        tracing::info!("Greeter shutting down");
        Ok(())
    }
}

#[get("/hello")]
async fn hello(greeter: Arc<Greeter>) -> impl IntoResponse {
    Json(serde_json::json!({ "message": greeter.greet("world") }))
}

#[get("/hello/{name}")]
async fn hello_name(
    greeter: Arc<Greeter>,
    name: gearbox_rs::Path<String>,
) -> impl IntoResponse {
    Json(serde_json::json!({ "message": greeter.greet(&name) }))
}
