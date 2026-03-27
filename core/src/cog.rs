use crate::error::Error;
use crate::hub::Hub;
use async_trait::async_trait;
use std::sync::Arc;

/// A managed service component in the Gearbox framework.
///
/// Cogs are the building blocks of a Gearbox application. Each Cog is constructed
/// once at startup, stored in the [`Hub`] registry, and made available to route
/// handlers via the [`Inject<T>`](crate::Inject) extractor.
///
/// You rarely implement this trait by hand — the `#[cog]` proc macro generates
/// the implementation, the [`CogFactory`](crate::CogFactory), and the inventory
/// registration automatically.
///
/// # Example
///
/// ```ignore
/// #[cog]
/// pub struct UserService {
///     #[inject]
///     db: Arc<Database>,
///     #[config]
///     settings: UserConfig,
/// }
/// ```
#[async_trait]
pub trait Cog: Send + Sync + 'static {
    /// Construct this Cog, resolving dependencies and configuration from the [`Hub`].
    async fn new(hub: Arc<Hub>) -> Result<Self, Error>
    where
        Self: Sized;

    /// Called after all cogs have been initialized.
    /// Override to perform post-initialization work (start background tasks, warm caches, etc.).
    async fn on_start(&self) -> Result<(), Error> {
        Ok(())
    }

    /// Called during graceful shutdown.
    /// Override to perform cleanup (flush buffers, close connections, etc.).
    async fn on_shutdown(&self) -> Result<(), Error> {
        Ok(())
    }
}
