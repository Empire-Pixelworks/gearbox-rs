use std::sync::Arc;

use crate::cog::Cog;
use crate::config::{CogConfig, Config, GearboxAppConfig};
use crate::error::Error;
use crate::registry::CogRegistry;

/// Central hub holding the service registry and configuration.
///
/// The Hub is passed to all Cogs during construction and is available
/// as Axum state in route handlers.
pub struct Hub {
    pub(crate) registry: CogRegistry,
    pub(crate) config: Config,
}

impl Hub {
    /// Create a new Hub with the given configuration.
    pub fn new(config: Config) -> Self {
        Self {
            registry: CogRegistry::new(),
            config,
        }
    }

    /// Get a configuration by type.
    ///
    /// Convenience method that delegates to `self.config.get::<C>()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let db_config = hub.get_config::<DbConfig>();
    /// ```
    pub fn get_config<C: CogConfig + Clone>(&self) -> Result<C, Error> {
        self.config.get::<C>()
    }

    /// Get the framework app config.
    ///
    /// Convenience method that delegates to `self.config.app()`.
    ///
    /// # Example
    ///
    /// ```ignore
    /// let port = hub.app_config().http_port;
    /// let log_level = hub.app_config().log_level.clone();
    /// ```
    pub fn app_config(&self) -> &GearboxAppConfig {
        self.config.app()
    }

    /// Get a cog from the registry by type.
    ///
    /// Used by macro-generated code; prefer [`Inject`] in handlers.
    #[doc(hidden)]
    pub fn registry_get<C: Cog + 'static>(&self) -> Result<Arc<C>, Error> {
        self.registry.get::<C>()
    }

    /// Get a configuration by type.
    ///
    /// Used by macro-generated code; prefer [`get_config`](Self::get_config) in application code.
    #[doc(hidden)]
    pub fn config_get<C: CogConfig + Clone>(&self) -> Result<C, Error> {
        self.config.get::<C>()
    }

    /// Register a Cog instance into the registry.
    ///
    /// Useful for testing when you need to build a Cog with `Cog::new(hub)` and
    /// then register it in the same Hub (e.g. for handler tests with `Inject<T>`).
    pub fn registry_put<C: Cog + 'static>(&self, cog: C) -> Result<(), Error> {
        self.registry.put(cog)
    }
}
