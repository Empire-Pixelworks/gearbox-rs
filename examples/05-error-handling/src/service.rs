use crate::error::AppError;
use dashmap::DashMap;
use gearbox_rs::cog;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
}

#[derive(Deserialize)]
pub struct CreateItem {
    pub name: String,
}

#[cog]
pub struct ItemService {
    items: DashMap<String, Item>,
}

impl ItemService {
    /// Looks up an item by ID, returning `NotFound` if it doesn't exist.
    pub fn get_item(&self, id: &str) -> Result<Item, AppError> {
        self.items
            .get(id)
            .map(|r| r.clone())
            .ok_or_else(|| AppError::NotFound(format!("item '{}' not found", id)))
    }

    /// Creates a new item, returning `BadRequest` if the name is empty.
    pub fn create_item(&self, input: CreateItem) -> Result<Item, AppError> {
        if input.name.trim().is_empty() {
            return Err(AppError::BadRequest("name must not be empty".into()));
        }

        let item = Item {
            id: format!("{:08x}", rand_id()),
            name: input.name,
        };
        self.items.insert(item.id.clone(), item.clone());
        Ok(item)
    }

    pub fn list_items(&self) -> Vec<Item> {
        self.items.iter().map(|r| r.value().clone()).collect()
    }
}

fn rand_id() -> u32 {
    use std::sync::atomic::{AtomicU32, Ordering};
    static COUNTER: AtomicU32 = AtomicU32::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}
