use thiserror::Error;

#[derive(Debug, Error)]
pub enum PgError {
    #[error("{0}")]
    PoolInitializeError(String),
    #[error("{0}")]
    ConnectionFailed(String),
    #[error("{0}")]
    MigrationFailed(String),
    #[error("PSQL Error")]
    PsqlError(#[from] sqlx::Error),
    #[error("Entity not found")]
    NotFound,
}