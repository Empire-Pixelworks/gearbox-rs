use std::sync::Arc;

use crate::cog::Cog;
use crate::config::{CogConfig, Config, GearboxAppConfig};
use crate::hub::Hub;
use crate::registry::CogRegistry;

/// Builder for constructing a [`Hub`] with specific services and configs,
/// bypassing inventory-based discovery. Useful for unit and integration tests.
///
/// # Example
///
/// ```ignore
/// let hub = TestHubBuilder::new()
///     .with_config(MyConfig { port: 9090 })
///     .with_service(my_service)
///     .build();
///
/// let result = hub.get_config::<MyConfig>().unwrap();
/// assert_eq!(result.port, 9090);
/// ```
pub struct TestHubBuilder {
    registry: CogRegistry,
    config: Config,
}

impl TestHubBuilder {
    pub fn new() -> Self {
        Self {
            registry: CogRegistry::new(),
            config: Config::default(),
        }
    }

    /// Override the framework's `GearboxAppConfig` (http_port, log_level, etc.).
    pub fn with_app_config(mut self, app: GearboxAppConfig) -> Self {
        self.config.set_app(app);
        self
    }

    /// Register a typed `CogConfig` directly, bypassing TOML/env loading.
    pub fn with_config<C: CogConfig + Clone + 'static>(self, config: C) -> Self {
        self.config.insert(config);
        self
    }

    /// Register a pre-built `Cog` instance into the registry.
    pub fn with_service<T: Cog + 'static>(self, service: T) -> Self {
        self.registry
            .put(service)
            .expect("failed to register test service");
        self
    }

    /// Consume the builder and produce an `Arc<Hub>`.
    pub fn build(self) -> Arc<Hub> {
        Arc::new(Hub {
            registry: self.registry,
            config: self.config,
        })
    }
}

impl Default for TestHubBuilder {
    fn default() -> Self {
        Self::new()
    }
}
