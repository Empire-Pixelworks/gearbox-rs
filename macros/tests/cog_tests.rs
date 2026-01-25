//! Integration tests for the #[cog] macro with default attributes.

use gearbox_rs_core::{Cog, CogConfig, Config, Error, Hub};
use gearbox_rs_macros::{cog, cog_config};
use serde::Deserialize;
use std::sync::Arc;

// === Sync default function tests ===

fn empty_strings() -> Vec<String> {
    Vec::new()
}

fn initial_count() -> u32 {
    42
}

#[cog]
pub struct SyncDefaultCog {
    #[default(empty_strings)]
    items: Vec<String>,
    #[default(initial_count)]
    count: u32,
}

#[tokio::test]
async fn test_sync_default_fn() {
    let hub = Arc::new(Hub::new(Config::default()));
    let cog = SyncDefaultCog::new(hub).await.unwrap();

    assert!(cog.items.is_empty());
    assert_eq!(cog.count, 42);
}

// === Async default function tests ===

async fn create_async_value(hub: Arc<Hub>) -> Result<String, Error> {
    // Access hub to prove we have it
    let _ = &hub.config;
    Ok("async_initialized".to_string())
}

async fn create_numbers(_hub: Arc<Hub>) -> Result<Vec<i32>, Error> {
    Ok(vec![1, 2, 3])
}

#[cog]
pub struct AsyncDefaultCog {
    #[default_async(create_async_value)]
    value: String,
    #[default_async(create_numbers)]
    numbers: Vec<i32>,
}

#[tokio::test]
async fn test_async_default_fn() {
    let hub = Arc::new(Hub::new(Config::default()));
    let cog = AsyncDefaultCog::new(hub).await.unwrap();

    assert_eq!(cog.value, "async_initialized");
    assert_eq!(cog.numbers, vec![1, 2, 3]);
}

// === Mixed attributes test ===

fn default_name() -> String {
    "default_user".to_string()
}

async fn async_init_data(_hub: Arc<Hub>) -> Result<Vec<u8>, Error> {
    Ok(vec![0xDE, 0xAD, 0xBE, 0xEF])
}

#[cog]
pub struct MixedAttributesCog {
    #[default(default_name)]
    name: String,
    #[default_async(async_init_data)]
    data: Vec<u8>,
    regular_field: u64, // Uses Default::default()
}

#[tokio::test]
async fn test_mixed_attributes() {
    let hub = Arc::new(Hub::new(Config::default()));
    let cog = MixedAttributesCog::new(hub).await.unwrap();

    assert_eq!(cog.name, "default_user");
    assert_eq!(cog.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
    assert_eq!(cog.regular_field, 0);
}

// === Error propagation test ===

async fn failing_init(_hub: Arc<Hub>) -> Result<String, Error> {
    Err(Error::ServerError("initialization failed".to_string()))
}

#[cog]
pub struct FailingCog {
    #[default_async(failing_init)]
    value: String,
}

#[tokio::test]
async fn test_async_default_error_propagation() {
    let hub = Arc::new(Hub::new(Config::default()));
    let result = FailingCog::new(hub).await;

    assert!(result.is_err());
    match result {
        Err(Error::ServerError(msg)) => assert_eq!(msg, "initialization failed"),
        _ => panic!("Expected ServerError"),
    }
}

// === Module path function test ===

mod utils {
    use gearbox_rs_core::{Error, Hub};
    use std::sync::Arc;

    pub fn create_id() -> String {
        "module_id".to_string()
    }

    pub async fn async_create_id(_hub: Arc<Hub>) -> Result<String, Error> {
        Ok("async_module_id".to_string())
    }
}

#[cog]
pub struct ModulePathCog {
    #[default(utils::create_id)]
    sync_id: String,
    #[default_async(utils::async_create_id)]
    async_id: String,
}

#[tokio::test]
async fn test_module_path_functions() {
    let hub = Arc::new(Hub::new(Config::default()));
    let cog = ModulePathCog::new(hub).await.unwrap();

    assert_eq!(cog.sync_id, "module_id");
    assert_eq!(cog.async_id, "async_module_id");
}

// === CogConfig macro tests ===

#[cog_config("test-config")]
#[derive(Default, Deserialize)]
pub struct TestConfig {
    #[allow(dead_code)]
    value: String,
}

#[test]
fn test_cog_config_macro() {
    assert_eq!(TestConfig::CONFIG_KEY, "test-config");
}

#[cog_config("database")]
#[derive(Default, Deserialize)]
pub struct DbConfig {
    #[allow(dead_code)]
    url: String,
    #[allow(dead_code)]
    max_connections: u32,
}

#[test]
fn test_cog_config_macro_database() {
    assert_eq!(DbConfig::CONFIG_KEY, "database");
}
