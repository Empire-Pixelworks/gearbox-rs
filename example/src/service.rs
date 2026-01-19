use std::sync::Arc;
use serde::Deserialize;
use gearbox_macros::cog;
use crate::repo::{Database, User};

#[derive(Deserialize)]
pub struct CreateUser {
    name: String,
    email: String,
}

#[cog]
pub struct UserService {
    #[inject]
    db: Arc<Database>,
}

impl UserService {
    pub fn create_user(&self, input: CreateUser) -> User {
        let user = User {
            id: uuid::Uuid::new_v4().to_string(),
            name: input.name,
            email: input.email,
        };
        self.db.insert(user.clone());
        user
    }

    pub fn get_user(&self, id: &str) -> Option<User> {
        self.db.get(id)
    }

    pub fn list_users(&self) -> Vec<User> {
        self.db.all()
    }
}