use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
pub enum PgError {
    #[error("Pool initialization failed: {0}")]
    PoolInitializeError(String),

    #[error("Database connection failed")]
    ConnectionFailed(#[source] sqlx::Error),

    #[error("Migration failed")]
    MigrationFailed(#[from] sqlx::migrate::MigrateError),

    #[error("PSQL Error")]
    PsqlError(#[from] sqlx::Error),

    #[error("No pool registered for schema '{0}'")]
    SchemaNotFound(String),

    #[error("Entity not found")]
    NotFound,
}
