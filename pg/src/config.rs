use crate::error::PgError;
use gearbox_rs_macros::cog_config;
use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Pool, Postgres};
use std::collections::HashMap;
use std::path::Path;

/// Configuration for a single PostgreSQL schema, including its name and migration path.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaConfig {
    pub schema_name: String,
    #[serde(default)]
    pub migration_path: String,
}

/// Top-level PostgreSQL configuration, loaded from the `[postgres]` section of `config.toml`.
///
/// Supports both single-schema (legacy) and multi-schema setups. For multi-schema,
/// populate the [`schemas`](PgConfig::schemas) map; otherwise the top-level
/// `schema_name` and `migration_path` are used as a single `"default"` schema.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[cog_config("postgres")]
pub struct PgConfig {
    #[serde(default = "default_db_url")]
    pub database_url: String,
    #[serde(default = "default_db_username")]
    pub database_username: String,
    #[serde(default = "default_db_password")]
    pub database_password: String,
    #[serde(default = "default_schema_name")]
    pub schema_name: String,
    #[serde(default = "default_max_connections")]
    pub max_connections: u32,
    #[serde(default = "default_migration_path")]
    pub migration_path: String,
    #[serde(default)]
    pub schemas: HashMap<String, SchemaConfig>,
}

fn default_db_url() -> String {
    "localhost:5432".to_string()
}
fn default_db_username() -> String {
    "postgres".to_string()
}
fn default_db_password() -> String {
    "postgres".to_string()
}
fn default_schema_name() -> String {
    "public".to_string()
}
fn default_max_connections() -> u32 {
    5
}
fn default_migration_path() -> String {
    "./migrations".to_string()
}

impl PgConfig {
    /// Builds a PostgreSQL connection string from the configured URL, username, and password.
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}",
            self.database_username, self.database_password, self.database_url
        )
    }

    /// Returns the effective schema map.
    ///
    /// If [`schemas`](PgConfig::schemas) is explicitly configured, returns it directly.
    /// Otherwise synthesizes a single `"default"` entry from the top-level
    /// `schema_name` and `migration_path`.
    pub fn resolved_schemas(&self) -> HashMap<String, SchemaConfig> {
        if !self.schemas.is_empty() {
            self.schemas.clone()
        } else {
            let mut map = HashMap::new();
            map.insert(
                "default".to_string(),
                SchemaConfig {
                    schema_name: self.schema_name.clone(),
                    migration_path: self.migration_path.clone(),
                },
            );
            map
        }
    }
}

/// Creates a connection pool for a single schema.
///
/// Runs migrations first (if a migration path is configured), then creates a pool
/// with `search_path` pinned to the schema on every new connection.
pub async fn create_pool_for_schema(
    config: &PgConfig,
    schema: &SchemaConfig,
) -> Result<PgPool, PgError> {
    migrate(config, schema).await?;

    let search_path_command = format!("SET search_path = '{}';", &schema.schema_name);

    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .after_connect(move |conn, _meta| {
            let search_path_command = search_path_command.clone();
            Box::pin(async move {
                conn.execute(search_path_command.as_str()).await?;
                Ok(())
            })
        })
        .connect(&config.connection_string())
        .await
        .map_err(PgError::ConnectionFailed)
}

async fn get_migration_conn(
    config: &PgConfig,
    schema: &SchemaConfig,
) -> Result<Pool<Postgres>, PgError> {
    let search_path_command = format!("SET search_path = '{}';", &schema.schema_name);
    let create_schema_command = format!("CREATE SCHEMA IF NOT EXISTS {};", &schema.schema_name);

    PgPoolOptions::new()
        .max_connections(1)
        .after_connect(move |conn, _meta| {
            let search_path_command = search_path_command.clone();
            let create_schema_command = create_schema_command.clone();
            Box::pin(async move {
                conn.execute(search_path_command.as_str()).await?;
                conn.execute(create_schema_command.as_str()).await?;
                Ok(())
            })
        })
        .connect(&config.connection_string())
        .await
        .map_err(PgError::ConnectionFailed)
}

async fn migrate(config: &PgConfig, schema: &SchemaConfig) -> Result<(), PgError> {
    if !schema.migration_path.is_empty() {
        let postgres_pool = get_migration_conn(config, schema).await?;
        Migrator::new(Path::new(&schema.migration_path))
            .await?
            .run(&postgres_pool)
            .await?;
        Ok(())
    } else {
        Ok(())
    }
}
