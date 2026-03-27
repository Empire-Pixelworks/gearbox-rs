use gearbox_rs::prelude::*;
use serde::Deserialize;

#[cog_config("greeting")]
#[derive(Default, Clone, Deserialize)]
#[serde(default)]
pub struct GreetingConfig {
    pub message: String,
}

#[cog]
pub struct GreetingService {
    #[config]
    config: GreetingConfig,
}

impl GreetingService {
    pub fn greet(&self, name: &str) -> String {
        if self.config.message.is_empty() {
            format!("Hello, {}!", name)
        } else {
            format!("{}, {}!", self.config.message, name)
        }
    }
}

#[get("/greet/:name")]
pub async fn greet_handler(
    name: Path<String>,
    service: Arc<GreetingService>,
) -> impl IntoResponse {
    service.greet(&name)
}
