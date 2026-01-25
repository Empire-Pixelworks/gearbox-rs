use crate::config::{CogConfig, Config, GearboxAppConfig};
use crate::registry::CogRegistry;

/// Central hub holding the service registry and configuration.
///
/// The Hub is passed to all Cogs during construction and is available
/// as Axum state in route handlers.
pub struct Hub {
    pub registry: CogRegistry,
    pub config: Config,
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
    pub fn get_config<C: CogConfig + Clone>(&self) -> C {
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
}
