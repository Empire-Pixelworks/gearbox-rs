use std::sync::Arc;
use async_trait::async_trait;
use crate::error::Error;
use crate::hub::Hub;

#[async_trait]
pub trait Cog: Send + Sync + 'static {
    async fn new(hub: Arc<Hub>) -> Result<Self, Error> where Self: Sized;
}