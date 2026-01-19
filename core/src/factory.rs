use std::any::{Any, TypeId};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Error;
use crate::hub::Hub;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

pub trait CogFactory: Send + Sync + 'static {
    fn type_id(&self) -> TypeId;
    fn type_name(&self) -> &'static str;
    fn deps(&self) -> Vec<TypeId>;
    fn build(&self, hub: Arc<Hub>) -> BoxFuture<'static, Result<Arc<dyn Any + Send + Sync>, Error>>;
}

inventory::collect!(&'static dyn CogFactory);
