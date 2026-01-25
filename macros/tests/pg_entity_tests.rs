//! Integration tests for the #[derive(PgEntity)] macro.
//!
//! These tests verify that the macro generates correct implementations
//! for the PgEntity trait. Note that actual database tests would require
//! a running Postgres instance.

use gearbox_rs_macros::PgEntity;
use gearbox_rs_postgres::PgEntity;

// === Basic single primary key ===

#[derive(PgEntity)]
#[table("users")]
pub struct User {
    #[primary_key]
    pub id: String,
    pub name: String,
    pub email: String,
}

#[test]
fn test_user_entity_constants() {
    assert_eq!(User::TABLE, "users");
    assert_eq!(User::COLUMNS, &["id", "name", "email"]);
    assert_eq!(User::PK_COLUMNS, &["id"]);
}

#[test]
fn test_user_id_extraction() {
    let user = User {
        id: "user-123".to_string(),
        name: "Test User".to_string(),
        email: "test@example.com".to_string(),
    };
    assert_eq!(user.id(), "user-123");
}

// === Schema-qualified table name ===

#[derive(PgEntity)]
#[table("public.products")]
pub struct Product {
    #[primary_key]
    pub id: i64,
    pub name: String,
    pub price: f64,
}

#[test]
fn test_product_schema_qualified_table() {
    assert_eq!(Product::TABLE, "public.products");
    assert_eq!(Product::PK_COLUMNS, &["id"]);
}

// === Skip attribute ===

#[derive(PgEntity)]
#[table("documents")]
pub struct Document {
    #[primary_key]
    pub id: String,
    pub content: String,
    #[skip]
    pub computed_hash: String,
}

#[test]
fn test_skip_field_excluded_from_columns() {
    // computed_hash should not be in COLUMNS
    assert_eq!(Document::COLUMNS, &["id", "content"]);
    assert!(!Document::COLUMNS.contains(&"computed_hash"));
}

#[test]
fn test_skip_field_defaults() {
    let doc = Document {
        id: "doc-1".to_string(),
        content: "Hello".to_string(),
        computed_hash: "should-be-empty-from-db".to_string(),
    };
    // The skip field is still accessible on the struct
    assert_eq!(doc.computed_hash, "should-be-empty-from-db");
}

// === Composite primary key ===

#[derive(PgEntity)]
#[table("order_items")]
pub struct OrderItem {
    #[primary_key]
    pub order_id: i64,
    #[primary_key]
    pub product_id: i64,
    pub quantity: i32,
}

#[test]
fn test_composite_key_columns() {
    assert_eq!(OrderItem::TABLE, "order_items");
    assert_eq!(OrderItem::COLUMNS, &["order_id", "product_id", "quantity"]);
    assert_eq!(OrderItem::PK_COLUMNS, &["order_id", "product_id"]);
}

#[test]
fn test_composite_key_extraction() {
    let item = OrderItem {
        order_id: 100,
        product_id: 42,
        quantity: 5,
    };
    let id: (i64, i64) = item.id();
    assert_eq!(id, (100, 42));
}

// === Skip upsert attribute ===

#[derive(PgEntity)]
#[table("counters")]
pub struct Counter {
    #[primary_key]
    pub id: String,
    pub value: i64,
    #[skip_upsert]
    pub created_at: String,
}

#[test]
fn test_skip_upsert_still_in_columns() {
    // skip_upsert fields are still in COLUMNS (they're inserted)
    assert_eq!(Counter::COLUMNS, &["id", "value", "created_at"]);
}

// === Mixed types composite key ===

#[derive(PgEntity)]
#[table("user_roles")]
pub struct UserRole {
    #[primary_key]
    pub user_id: i64,
    #[primary_key]
    pub role_name: String,
    pub granted_at: String,
}

#[test]
fn test_mixed_type_composite_key() {
    let role = UserRole {
        user_id: 1,
        role_name: "admin".to_string(),
        granted_at: "2024-01-01".to_string(),
    };
    let id: (i64, String) = role.id();
    assert_eq!(id, (1, "admin".to_string()));
}

// === All attributes combined ===

#[derive(PgEntity)]
#[table("complex_entities")]
pub struct ComplexEntity {
    #[primary_key]
    pub pk1: String,
    #[primary_key]
    pub pk2: i32,
    pub regular_field: String,
    #[skip_upsert]
    pub immutable_field: String,
    #[skip]
    pub computed: Vec<u8>,
}

#[test]
fn test_complex_entity() {
    assert_eq!(ComplexEntity::TABLE, "complex_entities");
    assert_eq!(
        ComplexEntity::COLUMNS,
        &["pk1", "pk2", "regular_field", "immutable_field"]
    );
    assert_eq!(ComplexEntity::PK_COLUMNS, &["pk1", "pk2"]);
}
