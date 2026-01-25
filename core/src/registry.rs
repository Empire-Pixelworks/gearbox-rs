use std::any::{type_name, Any, TypeId};
use std::sync::Arc;
use dashmap::DashMap;
use crate::cog::Cog;
use crate::error::Error;
use crate::error::Error::{CogDowncastFailed, CogNotFound, CogRegisterError};

pub struct CogRegistry {
    modules: DashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl Default for CogRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl CogRegistry {
    pub fn new() -> Self {
        Self {
            modules: DashMap::new(),
        }
    }

    pub fn get<C: Cog + 'static>(&self) -> Result<Arc<C>, Error> {
        let cog = self.modules.get(&TypeId::of::<C>())
            .ok_or(CogNotFound(type_name::<C>().to_string()))?;

        cog.value().clone().downcast::<C>().map_err(|_| CogDowncastFailed(type_name::<C>().to_string()))
    }

    pub fn put<C: Cog + 'static>(&self, cog: C) -> Result<(), Error> {
        let _ = self.modules.insert(TypeId::of::<C>(), Arc::new(cog))
            .ok_or(CogRegisterError(type_name::<C>().to_string()))?;

        Ok(())
    }

    pub fn put_any(&self, type_id: TypeId, cog: Arc<dyn Any + Send + Sync>) {
        self.modules.insert(type_id, cog);
    }
}