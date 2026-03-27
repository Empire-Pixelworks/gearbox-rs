use std::any::{Any, TypeId};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use crate::error::Error;
use crate::hub::Hub;

/// Convenience alias for a pinned, boxed, `Send` future.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Factory for constructing and managing a single [`Cog`](crate::Cog) type.
///
/// A `CogFactory` is generated automatically by the `#[cog]` proc macro and
/// registered via [`inventory`]. You should never need to implement this trait
/// by hand.
///
/// This type is `#[doc(hidden)]` from the public API.
pub trait CogFactory: Send + Sync + 'static {
    /// Returns the [`TypeId`] of the Cog this factory produces.
    fn type_id(&self) -> TypeId;

    /// Returns the human-readable type name, used in error messages and logging.
    fn type_name(&self) -> &'static str;

    /// Returns the [`TypeId`]s of all `#[inject]` dependencies.
    ///
    /// Used by [`resolve_init_order`](crate::resolve_init_order) to perform
    /// topological sorting of the initialization graph.
    fn deps(&self) -> Vec<TypeId>;

    /// Construct the Cog asynchronously, given access to the [`Hub`].
    fn build(&self, hub: Arc<Hub>)
    -> BoxFuture<'static, Result<Arc<dyn Any + Send + Sync>, Error>>;

    /// Called after all Cogs have been built. Default is a no-op.
    ///
    /// Overridden when the `#[cog]` macro sees an `#[on_start(...)]` attribute.
    fn on_start(&self, _cog: Arc<dyn Any + Send + Sync>) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }

    /// Called during graceful shutdown. Default is a no-op.
    ///
    /// Overridden when the `#[cog]` macro sees an `#[on_shutdown(...)]` attribute.
    fn on_shutdown(&self, _cog: Arc<dyn Any + Send + Sync>) -> BoxFuture<'static, Result<(), Error>> {
        Box::pin(async { Ok(()) })
    }
}

// Enable auto-discovery of CogFactory implementations via `inventory`.
inventory::collect!(&'static dyn CogFactory);
