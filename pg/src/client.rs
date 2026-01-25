use std::sync::Arc;
use sqlx::PgPool;
use thiserror::Error;
use gearbox_core::{Error, Hub};
use gearbox_macros::cog;

#[cog]
pub struct PgClient {
    #[default_async(load_pool)]
    pub pool: Arc<PgPool>,
}

async fn load_pool(hub: Arc<Hub>) -> Result<Arc<PgPool>, Error> {
    todo!()
}

#[derive(Clone, Debug, Error)]
pub enum PgError {
    #[error("{0}")]
    ConnectionFailed(String),
    #[error("{0}")]
    MigrationFailed(String),
}