mod app;
mod cog;
mod config;
mod crud;
mod pg_entity;
mod pg_queries;
mod route;

use proc_macro::TokenStream;

/// Derive macro for creating a Cog with automatic dependency injection.
///
/// Apply this attribute to a struct to automatically generate:
/// - `impl Cog` with a `new(hub)` constructor
/// - A factory struct implementing `CogFactory`
/// - Automatic registration with the inventory system
///
/// # Field Attributes
///
/// - `#[inject]` - Marks a field as a dependency to be injected from the registry.
///   The field type must be `Arc<T>` where `T: Cog`.
/// - `#[config]` - Marks a field to be loaded from configuration.
///   The field type must implement `CogConfig + Default`.
/// - `#[default(fn)]` - Marks a field to be initialized via a sync function.
///   The function must have signature `fn() -> T`.
/// - `#[default_async(fn)]` - Marks a field to be initialized via an async function.
///   The function must have signature `async fn(&Arc<Hub>) -> Result<T, Error>`.
/// - No attribute - Field must implement `Default` and will use `Default::default()`.
///
/// # Example
///
/// ```ignore
/// use std::sync::Arc;
/// use gearbox_rs_macros::cog;
///
/// #[cog]
/// struct UserService {
///     #[inject]
///     db: Arc<Database>,
///     #[inject]
///     cache: Arc<CacheClient>,
///     #[config]
///     settings: UserServiceConfig,
///     request_count: u64,  // Uses Default::default()
/// }
/// ```
///
/// # Custom Default Functions
///
/// ```ignore
/// fn empty_users() -> Vec<String> {
///     Vec::new()
/// }
///
/// async fn create_pool(hub: &Arc<Hub>) -> Result<PgPool, Error> {
///     let config = hub.config.get::<DbConfig>();
///     PgPool::connect(&config.url).await
///         .map_err(|e| Error::ServerError(e.to_string()))
/// }
///
/// #[cog]
/// struct DataService {
///     #[default(empty_users)]
///     users: Vec<String>,
///     #[default_async(create_pool)]
///     pool: PgPool,
/// }
/// ```
#[proc_macro_attribute]
pub fn cog(_attr: TokenStream, item: TokenStream) -> TokenStream {
    cog::generate_cog(item)
}

/// Defines a GET route handler with automatic dependency injection.
///
/// # Example
///
/// ```ignore
/// #[get("/users")]
/// async fn list_users(repo: Arc<UserRepo>) -> impl IntoResponse {
///     Json(repo.find_all().await)
/// }
/// ```
#[proc_macro_attribute]
pub fn get(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("GET", attr, item)
}

/// Defines a POST route handler with automatic dependency injection.
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("POST", attr, item)
}

/// Defines a PUT route handler with automatic dependency injection.
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("PUT", attr, item)
}

/// Defines a DELETE route handler with automatic dependency injection.
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("DELETE", attr, item)
}

/// Defines a PATCH route handler with automatic dependency injection.
#[proc_macro_attribute]
pub fn patch(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("PATCH", attr, item)
}

/// Implements `CogConfig` trait for a struct with the given config key.
///
/// The struct must also derive `Default` and `serde::Deserialize`.
///
/// # Example
///
/// ```ignore
/// #[cog_config("database")]
/// #[derive(Default, Deserialize)]
/// pub struct DbConfig {
///     url: String,
///     max_connections: u32,
/// }
/// ```
#[proc_macro_attribute]
pub fn cog_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    config::generate_cog_config(attr, item)
}

/// Derive macro for implementing `PgEntity` and `PgRepository` traits.
///
/// Generates implementations that enable repository operations directly on `PgClient`.
///
/// # Attributes
///
/// ## Struct-level
/// - `#[table("name")]` - Required. Specifies the database table name (may include schema).
///
/// ## Field-level
/// - `#[primary_key]` - Marks field(s) as primary key. Multiple fields create composite keys.
/// - `#[skip]` - Excludes field from all database operations. Field must implement `Default`.
/// - `#[skip_upsert]` - Excludes field from UPDATE portion of upsert operations.
/// - `#[pg_type(Type)]` - Casts field to a different type when binding to queries.
///
/// # Example
///
/// ```ignore
/// use gearbox_rs_macros::PgEntity;
///
/// #[derive(PgEntity)]
/// #[table("users")]
/// pub struct User {
///     #[primary_key]
///     pub id: String,
///     pub name: String,
///     pub email: String,
///     #[skip]
///     pub computed_field: String,  // Not stored in DB
/// }
///
/// // Usage with PgClient:
/// let client: Arc<PgClient> = hub.get()?;
/// let user = client.create(user).await?;
/// let found = client.find_by_id::<User>(&id).await?;
/// ```
///
/// # Composite Keys
///
/// ```ignore
/// #[derive(PgEntity)]
/// #[table("order_items")]
/// pub struct OrderItem {
///     #[primary_key]
///     pub order_id: i64,
///     #[primary_key]
///     pub product_id: i64,
///     pub quantity: i32,
/// }
/// // Generated: type Id = (i64, i64);
/// // Usage: client.find_by_id::<OrderItem>(&(order_id, product_id)).await?;
/// ```
#[proc_macro_derive(PgEntity, attributes(table, primary_key, skip, skip_upsert, pg_type))]
pub fn pg_entity(input: TokenStream) -> TokenStream {
    pg_entity::generate_pg_entity(input)
}

/// Derive macro for generating REST CRUD endpoints with pagination.
///
/// Apply this alongside `PgEntity` to automatically generate:
/// - Create, Update, Query, and Response DTOs
/// - REST route handlers (GET, POST, PUT, PATCH, DELETE)
///
/// # Struct-level Attributes
///
/// - `#[table("name")]` - Required. Database table name (from PgEntity).
/// - `#[crud(path = "/users")]` - Optional. REST endpoint path (default: pluralized snake_case).
/// - `#[crud(read_only)]` - Optional. Generate only read endpoints (GET list/detail).
/// - `#[crud(skip_create)]` - Optional. Skip POST endpoint generation.
/// - `#[crud(skip_delete)]` - Optional. Skip DELETE endpoint generation.
///
/// # Field-level Attributes
///
/// - `#[primary_key]` - Marks field as primary key (from PgEntity).
/// - `#[auto_generated]` - Field is generated by DB (UUID, serial). Excluded from create/update DTOs.
/// - `#[readonly]` - Field is only in responses (e.g., `created_at`). Excluded from create/update DTOs.
/// - `#[writeonly]` - Field is only in create/update (e.g., `password_hash`). Excluded from responses.
///
/// # Example
///
/// ```ignore
/// use gearbox_rs_macros::{PgEntity, Crud};
/// use uuid::Uuid;
/// use chrono::{DateTime, Utc};
///
/// #[derive(PgEntity, Crud)]
/// #[table("users")]
/// #[crud(path = "/users")]
/// pub struct User {
///     #[primary_key]
///     #[auto_generated]
///     pub id: Uuid,
///
///     pub name: String,
///     pub email: String,
///
///     #[writeonly]
///     pub password_hash: String,
///
///     #[readonly]
///     pub created_at: DateTime<Utc>,
///
///     pub updated_at: DateTime<Utc>,
///     pub active: bool,
/// }
/// ```
///
/// This generates:
/// - `UserCreate` - DTO for POST requests
/// - `UserUpdate` - DTO for PATCH requests (all fields optional)
/// - `UserQuery` - Query parameters for pagination (`limit`, `offset`)
/// - `UserResponse` - DTO for responses (excludes `password_hash`)
/// - Route handlers registered with inventory
///
/// For custom filtering, use `pg_queries!` with a custom route handler.
///
/// # Generated Routes
///
/// | Method | Path | Description |
/// |--------|------|-------------|
/// | GET | `/{path}` | List with pagination |
/// | GET | `/{path}/{id}` | Get single entity |
/// | POST | `/{path}` | Create new entity |
/// | PUT | `/{path}/{id}` | Full update |
/// | PATCH | `/{path}/{id}` | Partial update |
/// | DELETE | `/{path}/{id}` | Delete entity |
#[proc_macro_derive(
    Crud,
    attributes(
        table,
        crud,
        primary_key,
        auto_generated,
        readonly,
        writeonly,
        skip,
        pg_type
    )
)]
pub fn crud(input: TokenStream) -> TokenStream {
    crud::generate_crud(input)
}

/// Generates a Gearbox application entry point.
///
/// This macro replaces the standard `main` function with the Gearbox startup sequence.
/// It handles tokio runtime setup and initializes the Gearbox framework.
///
/// # Example
///
/// ```ignore
/// use gearbox_rs_macros::gearbox_app;
///
/// #[gearbox_app]
/// fn main() {}
/// ```
///
/// This expands to:
///
/// ```ignore
/// #[tokio::main]
/// async fn main() -> Result<(), gearbox_rs_core::Error> {
///     gearbox_rs_core::Gearbox::crank().await?.ignite().await
/// }
/// ```
#[proc_macro_attribute]
pub fn gearbox_app(attr: TokenStream, item: TokenStream) -> TokenStream {
    app::generate_gearbox_app(attr, item)
}

/// Generate custom query methods on `PgClient`.
///
/// This macro allows you to define custom SQL queries with type-safe parameters
/// and return types. Methods are generated directly on `PgClient`.
///
/// # Syntax
///
/// ```ignore
/// pg_queries! {
///     fn function_name(param1: Type1, param2: Type2) -> ReturnType {
///         "SQL query with $1, $2 placeholders"
///     }
/// }
/// ```
///
/// # Return Types
///
/// | Return Type | Behavior |
/// |-------------|----------|
/// | `Option<T>` | `fetch_optional` - returns `None` if no row found |
/// | `Vec<T>` | `fetch_all` - returns all matching rows |
/// | `T` (struct) | `fetch_one` - returns exactly one row, errors if not found |
/// | `i64`, `String`, etc. | `query_scalar` - returns a single column value |
/// | `bool` | `execute` - returns `true` if rows_affected > 0 |
/// | `u64` | `execute` - returns rows_affected count |
/// | (none) | `execute` - returns `()` |
///
/// # Example
///
/// ```ignore
/// use gearbox_rs_macros::pg_queries;
///
/// // Define a summary struct (must derive sqlx::FromRow)
/// #[derive(sqlx::FromRow)]
/// pub struct UserSummary {
///     pub id: String,
///     pub name: String,
/// }
///
/// pg_queries! {
///     // Returns Option<T> - fetch_optional
///     fn find_user_by_email(email: &str) -> Option<User> {
///         "SELECT * FROM users WHERE email = $1"
///     }
///
///     // Returns Vec<T> - fetch_all
///     fn find_users_by_status(status: &str) -> Vec<User> {
///         "SELECT * FROM users WHERE status = $1"
///     }
///
///     // Returns custom struct
///     fn get_user_summary(id: &str) -> Option<UserSummary> {
///         "SELECT id, name FROM users WHERE id = $1"
///     }
///
///     // Returns scalar value
///     fn count_active_users() -> i64 {
///         "SELECT COUNT(*) FROM users WHERE active = true"
///     }
///
///     // Returns bool (rows_affected > 0)
///     fn deactivate_user(id: &str) -> bool {
///         "UPDATE users SET active = false WHERE id = $1"
///     }
///
///     // Returns rows affected
///     fn delete_inactive_users() -> u64 {
///         "DELETE FROM users WHERE active = false"
///     }
///
///     // No return - just execute
///     fn log_access(user_id: &str, action: &str) {
///         "INSERT INTO audit_log (user_id, action) VALUES ($1, $2)"
///     }
/// }
///
/// // Usage:
/// let client: Arc<PgClient> = hub.registry.get()?;
/// let user = client.find_user_by_email("test@example.com").await?;
/// let count = client.count_active_users().await?;
/// ```
///
/// # Notes
///
/// - Return types must implement `sqlx::FromRow` (for structs) or be scalar types
/// - Parameter count must match the number of `$N` placeholders in the SQL
/// - The macro validates placeholder count at compile time
#[proc_macro]
pub fn pg_queries(input: TokenStream) -> TokenStream {
    pg_queries::pg_queries(input)
}
