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

    #[searchable]
    pub name: String,

    #[searchable(eq)]
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

    #[searchable]
    pub action: String,

    #[readonly]
    pub timestamp: i64,
}

// Entity with numeric searchable fields
#[derive(Clone, PgEntity, Crud)]
#[table("products")]
pub struct Product {
    #[primary_key]
    #[auto_generated]
    pub id: String,

    #[searchable]
    pub name: String,

    #[searchable]
    pub price: i64,

    #[searchable]
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
    // UserQuery should have pagination fields and searchable fields with operators
    let query = UserQuery {
        limit: Some(10),
        offset: Some(0),
        sort: Some("-created_at".to_string()),
        name: Some("John".to_string()),
        name_like: Some("John".to_string()),
        name_starts_with: Some("J".to_string()),
        email: Some("john@example.com".to_string()),
    };
    assert_eq!(query.limit, Some(10));
    assert_eq!(query.name, Some("John".to_string()));
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

#[test]
fn test_product_query_numeric_operators() {
    // Product should have numeric operators for price and stock
    let query = ProductQuery {
        limit: None,
        offset: None,
        sort: None,
        name: None,
        name_like: None,
        name_starts_with: None,
        price: None,
        price_gt: Some(100),
        price_gte: None,
        price_lt: Some(1000),
        price_lte: None,
        stock: None,
        stock_gt: None,
        stock_gte: Some(10),
        stock_lt: None,
        stock_lte: None,
    };
    assert_eq!(query.price_gt, Some(100));
    assert_eq!(query.stock_gte, Some(10));
}

#[test]
fn test_build_where_clause() {
    use gearbox_rs_core::crud::BuildWhereClause;

    let query = UserQuery {
        limit: Some(10),
        offset: Some(20),
        sort: Some("-name".to_string()),
        name: Some("John".to_string()),
        name_like: None,
        name_starts_with: None,
        email: None,
    };

    let (conditions, params) = query.build_conditions();
    assert_eq!(conditions.len(), 1);
    assert!(conditions[0].contains("name"));
    assert_eq!(params.len(), 1);
    assert_eq!(params[0], "John");

    let (limit, offset) = query.pagination();
    assert_eq!(limit, Some(10));
    assert_eq!(offset, Some(20));

    let sort = query.sort_spec().expect("should have sort");
    assert_eq!(sort.column, "name");
    assert_eq!(sort.direction, gearbox_rs_core::crud::SortDirection::Desc);
}

#[test]
fn test_build_where_clause_like() {
    use gearbox_rs_core::crud::BuildWhereClause;

    let query = UserQuery {
        limit: None,
        offset: None,
        sort: None,
        name: None,
        name_like: Some("john".to_string()),
        name_starts_with: None,
        email: None,
    };

    let (conditions, params) = query.build_conditions();
    assert_eq!(conditions.len(), 1);
    assert!(conditions[0].contains("ILIKE"));
    assert_eq!(params[0], "%john%"); // LIKE wraps with %
}

#[test]
fn test_build_where_clause_starts_with() {
    use gearbox_rs_core::crud::BuildWhereClause;

    let query = UserQuery {
        limit: None,
        offset: None,
        sort: None,
        name: None,
        name_like: None,
        name_starts_with: Some("J".to_string()),
        email: None,
    };

    let (conditions, params) = query.build_conditions();
    assert_eq!(conditions.len(), 1);
    assert!(conditions[0].contains("ILIKE"));
    assert_eq!(params[0], "J%"); // starts_with appends %
}
