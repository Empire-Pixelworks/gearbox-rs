use std::sync::Arc;
use sqlx::PgPool;
use gearbox_rs_core::{Error, Hub};
use gearbox_rs_macros::cog;
use crate::config::{init_schema_pool, PgConfig};

#[cog]
pub struct PgClient {
    #[default_async(load_pool)]
    #[allow(dead_code)]
    pub pool: Arc<PgPool>,
}

async fn load_pool(hub: Arc<Hub>) -> Result<Arc<PgPool>, Error> {
    let config = hub.config.get::<PgConfig>();
    Ok(
        Arc::new(
            init_schema_pool(&config).await
                .map_err(|e| Error::ServerError(format!("{:?}", e)))?
        )
    )
}
