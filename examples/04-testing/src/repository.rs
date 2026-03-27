use gearbox_rs::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
}

/// A simple in-memory repository cog.
///
/// In production, this would wrap a `PgClient` and query a real database.
/// For testing, use [`ItemRepository::with_items`] to pre-populate data
/// and register it via `hub.registry_put()`.
#[cog]
pub struct ItemRepository {
    items: RwLock<HashMap<String, Item>>,
}

impl ItemRepository {
    /// Create a repository pre-loaded with items (for testing).
    pub fn with_items(items: Vec<Item>) -> Self {
        let map = items.into_iter().map(|i| (i.id.clone(), i)).collect();
        Self {
            items: RwLock::new(map),
        }
    }

    pub fn get(&self, id: &str) -> Option<Item> {
        self.items.read().unwrap().get(id).cloned()
    }

    pub fn list(&self) -> Vec<Item> {
        self.items.read().unwrap().values().cloned().collect()
    }
}
