use dashmap::DashMap;
use serde::{Deserialize, Serialize};
use gearbox_rs_macros::cog;

#[derive(Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub name: String,
    pub email: String,
}

#[cog]
pub struct Database {
    users: DashMap<String, User>,
}

impl Database {
    pub fn insert(&self, user: User) {
        self.users.insert(user.id.clone(), user);
    }

    pub fn get(&self, id: &str) -> Option<User> {
        self.users.get(id).map(|r| r.clone())
    }

    pub fn all(&self) -> Vec<User> {
        self.users.iter().map(|r| r.value().clone()).collect()
    }
}