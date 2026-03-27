use crate::cog::Cog;
use crate::error::Error;
use crate::error::Error::{CogDowncastFailed, CogNotFound};
use dashmap::DashMap;
use std::any::{Any, TypeId, type_name};
use std::sync::Arc;

/// Type-erased service registry backed by [`DashMap`].
///
/// Stores Cog instances as `Arc<dyn Any + Send + Sync>` keyed by [`TypeId`].
/// Thread-safe for concurrent reads; writes happen only during startup.
///
/// This type is `#[doc(hidden)]` from the public API. Application code should
/// use [`Hub`](crate::Hub) methods or the [`Inject<T>`](crate::Inject) extractor
/// instead of accessing the registry directly.
pub struct CogRegistry {
    modules: DashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Default for CogRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CogRegistry {
    /// Creates an empty registry.
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
        }
    }

    /// Retrieves a Cog by type.
    ///
    /// Returns [`Error::CogNotFound`] if no Cog of type `C` is registered, or
    /// [`Error::CogDowncastFailed`] if the stored value cannot be downcast to `C`.
    pub fn get<C: Cog + 'static>(&self) -> Result<Arc<C>, Error> {
        let cog = self
            .modules
            .get(&TypeId::of::<C>())
            .ok_or(CogNotFound(type_name::<C>().to_string()))?;

        cog.value()
            .clone()
            .downcast::<C>()
            .map_err(|_| CogDowncastFailed(type_name::<C>().to_string()))
    }

    /// Registers a Cog instance, wrapping it in `Arc` internally.
    pub fn put<C: Cog + 'static>(&self, cog: C) -> Result<(), Error> {
        self.modules.insert(TypeId::of::<C>(), Arc::new(cog));
        Ok(())
    }

    /// Low-level insert for a pre-built type-erased Cog. Used by [`Gearbox::crank`](crate::Gearbox::crank).
    pub fn put_any(&self, type_id: TypeId, cog: Arc<dyn Any + Send + Sync>) {
        self.modules.insert(type_id, cog);
    }

    /// Low-level retrieval by [`TypeId`]. Used during shutdown to pass Cogs to factory callbacks.
    pub fn get_any(&self, type_id: &TypeId) -> Option<Arc<dyn Any + Send + Sync>> {
        self.modules.get(type_id).map(|entry| entry.value().clone())
    }
}
