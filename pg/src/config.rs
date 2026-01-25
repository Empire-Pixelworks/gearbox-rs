use serde::{Deserialize, Serialize};
use sqlx::migrate::Migrator;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool, Pool, Postgres};
use std::path::Path;
use gearbox_rs_macros::cog_config;
use crate::error::PgError;

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
    pub fn connection_string(&self) -> String {
        format!(
            "postgres://{}:{}@{}",
            self.database_username, self.database_password, self.database_url
        )
    }
}

pub async fn init_schema_pool(config: &PgConfig) -> Result<PgPool, PgError> {
    migrate(config).await?;

    PgPoolOptions::new()
        .max_connections(config.max_connections)
        .connect(&config.connection_string())
        .await
        .map_err(|e| PgError::ConnectionFailed(e.to_string()))
}

async fn get_migration_conn(config: &PgConfig) -> Result<Pool<Postgres>, PgError> {
    let search_path_command = format!("SET search_path = '{}';", &config.schema_name);
    let create_schema_command = format!("CREATE SCHEMA IF NOT EXISTS {};", &config.schema_name);

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
        .map_err(|e| PgError::ConnectionFailed(e.to_string()))
}

async fn migrate(config: &PgConfig) -> Result<(), PgError> {
    if !config.migration_path.is_empty() {
        let postgres_pool = get_migration_conn(config).await?;
        Ok(
            Migrator::new(Path::new(&config.migration_path))
                .await
                .map_err(|e| PgError::MigrationFailed(format!("{:?}", e)))?
                .run(&postgres_pool)
                .await
                .map_err(|e| PgError::MigrationFailed(e.to_string()))?
        )
    } else {
        //warn!("No migration path provided; proceeding without running migrations");
        Ok(())
    }
}
