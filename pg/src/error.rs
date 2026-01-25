use thiserror::Error;

#[derive(Clone, Debug, Error)]
pub enum PgError {
    #[error("{0}")]
    PoolInitializeError(String),
}