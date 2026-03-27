use crate::config::{PgConfig, init_schema_pool};
use gearbox_rs_core::{Error, Hub};
use gearbox_rs_macros::cog;
use sqlx::PgPool;
use std::sync::Arc;

#[cog]
pub struct PgClient {
    #[default_async(load_pool)]
    #[allow(dead_code)]
    pub pool: Arc<PgPool>,
}

async fn load_pool(hub: Arc<Hub>) -> Result<Arc<PgPool>, Error> {
    let config = hub.config.get::<PgConfig>();
    println!("config: {:?}", config);
    Ok(Arc::new(
        init_schema_pool(&config)
            .await
            .map_err(|e| Error::ServerError(format!("{:?}", e)))?,
    ))
}
