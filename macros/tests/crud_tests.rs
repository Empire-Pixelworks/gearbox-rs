//! Tests for the Crud derive macro.

use gearbox_rs_macros::{Crud, PgEntity};

// Basic entity for testing
#[derive(Clone, PgEntity, Crud)]
#[table("users")]
#[crud(path = "/users")]
pub struct User {
    #[primary_key]
    #[auto_generated]
    pub id: String,

    pub name: String,

    pub email: String,

    #[writeonly]
    pub password_hash: String,

    #[readonly]
    pub created_at: i64,

    pub active: bool,
}

// Entity with read_only config
#[derive(Clone, PgEntity, Crud)]
#[table("audit_logs")]
#[crud(path = "/logs", read_only)]
pub struct AuditLog {
    #[primary_key]
    #[auto_generated]
    pub id: String,

    pub action: String,

    #[readonly]
    pub timestamp: i64,
}

// Entity with numeric fields
#[derive(Clone, PgEntity, Crud)]
#[table("products")]
pub struct Product {
    #[primary_key]
    #[auto_generated]
    pub id: String,

    pub name: String,

    pub price: i64,

    pub stock: i32,
}

#[test]
fn test_user_create_dto_generated() {
    // UserCreate should not have id (auto_generated), created_at (readonly), or password_hash visible in response
    let create = UserCreate {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "hash123".to_string(),
        active: true,
    };
    assert_eq!(create.name, "Test User");
}

#[test]
fn test_user_update_dto_generated() {
    // UserUpdate should have all fields as Option for partial updates
    let update = UserUpdate {
        name: Some("Updated Name".to_string()),
        email: None,
        password_hash: None,
        active: None,
    };
    assert_eq!(update.name, Some("Updated Name".to_string()));
}

#[test]
fn test_user_query_dto_generated() {
    // UserQuery should have pagination fields only
    let query = UserQuery {
        limit: Some(10),
        offset: Some(0),
    };
    assert_eq!(query.limit, Some(10));
    assert_eq!(query.offset, Some(0));
}

#[test]
fn test_user_response_dto_generated() {
    // UserResponse should not have password_hash (writeonly)
    let user = User {
        id: "123".to_string(),
        name: "Test".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "secret".to_string(),
        created_at: 1234567890,
        active: true,
    };
    let response = UserResponse::from(user);
    assert_eq!(response.id, "123");
    assert_eq!(response.name, "Test");
    // password_hash should not be in response
}

#[test]
fn test_user_create_into_entity() {
    let create = UserCreate {
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
        password_hash: "hash123".to_string(),
        active: true,
    };

    // into_entity requires the auto-generated id
    let entity = create.into_entity("generated-id".to_string());
    assert_eq!(entity.id, "generated-id");
    assert_eq!(entity.name, "Test User");
}
