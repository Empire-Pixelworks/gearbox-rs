//! Demonstrates testing a Cog in isolation using TestHubBuilder.

use gearbox_rs::{Cog, TestHubBuilder};
use testing_example::service::{GreetingConfig, GreetingService};

#[tokio::test]
async fn test_greeting_with_default_config() {
    let hub = TestHubBuilder::new()
        .with_config(GreetingConfig::default())
        .build();

    let service = GreetingService::new(hub).await.unwrap();
    assert_eq!(service.greet("World"), "Hello, World!");
}

#[tokio::test]
async fn test_greeting_with_custom_config() {
    let hub = TestHubBuilder::new()
        .with_config(GreetingConfig {
            message: "Howdy".to_string(),
        })
        .build();

    let service = GreetingService::new(hub).await.unwrap();
    assert_eq!(service.greet("Rustacean"), "Howdy, Rustacean!");
}
