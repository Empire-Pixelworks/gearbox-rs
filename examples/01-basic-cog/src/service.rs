use crate::repo::{Database, User};
use gearbox_rs_macros::cog;
use serde::Deserialize;
use std::sync::Arc;

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
