use crate::config::{PgConfig, create_pool_for_schema};
use gearbox_rs_core::{Error, Hub};
use gearbox_rs_macros::cog;
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;

/// PostgreSQL client Cog that manages a map of connection pools keyed by schema name.
///
/// Created automatically from [`PgConfig`](crate::config::PgConfig) during
/// `Gearbox::crank()`. Inject it into other Cogs or route handlers to access
/// the database:
///
/// ```ignore
/// #[cog]
/// pub struct UserRepo {
///     #[inject]
///     pg: Arc<PgClient>,
/// }
/// ```
#[cog]
pub struct PgClient {
    #[default_async(load_pools)]
    pub pools: Arc<HashMap<String, PgPool>>,
}

impl PgClient {
    /// Returns the connection pool for the given schema name.
    ///
    /// Returns [`PgError::SchemaNotFound`](crate::error::PgError::SchemaNotFound)
    /// if no pool is configured for that schema.
    pub fn get_pool(&self, schema: &str) -> Result<&PgPool, crate::error::PgError> {
        self.pools
            .get(schema)
            .ok_or_else(|| crate::error::PgError::SchemaNotFound(schema.to_string()))
    }

    /// Convenience method for single-schema setups.
    ///
    /// If exactly one pool exists, returns it. Otherwise looks up the `"default"` pool.
    /// Returns [`PgError::SchemaNotFound`](crate::error::PgError::SchemaNotFound)
    /// if no suitable pool is found.
    pub fn pool(&self) -> Result<&PgPool, crate::error::PgError> {
        if self.pools.len() == 1 {
            self.pools
                .values()
                .next()
                .ok_or_else(|| crate::error::PgError::SchemaNotFound("default".to_string()))
        } else {
            self.get_pool("default")
        }
    }
}

async fn load_pools(hub: Arc<Hub>) -> Result<Arc<HashMap<String, PgPool>>, Error> {
    let config = hub.get_config::<PgConfig>()?;
    let schemas = config.resolved_schemas();
    let mut pools = HashMap::new();

    for (key, schema) in &schemas {
        let pool = create_pool_for_schema(&config, schema)
            .await
            .map_err(|e| Error::External {
                context: format!("Failed to create pool for schema '{}'", key),
                source: Box::new(e),
            })?;
        pools.insert(key.clone(), pool);
    }

    Ok(Arc::new(pools))
}
