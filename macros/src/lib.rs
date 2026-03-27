mod app;
mod cog;
mod config;
mod crud;
mod paths;
mod pg_entity;
mod pg_queries;
mod route;
mod utils;

use proc_macro::TokenStream;

/// Derive macro for creating a Cog with automatic dependency injection.
///
/// Generates `impl Cog`, a `CogFactory`, and inventory registration.
///
/// # Field Attributes
///
/// - `#[inject]` - Dependency injection. Field type must be `Arc<T>` where `T: Cog`.
/// - `#[config]` - Load from configuration. Type must implement `CogConfig + Default`.
/// - `#[default(fn)]` - Initialize via sync function: `fn() -> T`.
/// - `#[default_async(fn)]` - Initialize via async function: `async fn(&Arc<Hub>) -> Result<T, Error>`.
/// - No attribute - Uses `Default::default()`.
///
/// # Example
///
/// ```ignore
/// #[cog]
/// struct UserService {
///     #[inject]
///     db: Arc<Database>,
///     #[config]
///     settings: UserServiceConfig,
///     request_count: u64,
/// }
/// ```
#[proc_macro_attribute]
pub fn cog(_attr: TokenStream, item: TokenStream) -> TokenStream {
    cog::generate_cog(item)
}

/// Defines a GET route handler with automatic dependency injection.
///
/// # Parameter Injection
///
/// Parameters typed as `Arc<T>` where `T: Cog` are automatically rewritten to use
/// `Inject<T>`, which extracts the service from the Hub at request time. Other
/// axum extractors (`Json`, `Path`, `Query`, etc.) pass through unchanged.
///
/// For example, `repo: Arc<UserRepo>` expands to `Inject(repo): Inject<UserRepo>`.
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
///
/// `Arc<T>` parameters are rewritten to `Inject<T>` (see [`get`] for details).
///
/// # Example
///
/// ```ignore
/// #[post("/users")]
/// async fn create_user(
///     repo: Arc<UserRepo>,
///     body: Json<CreateUser>,
/// ) -> impl IntoResponse {
///     let user = repo.create(body.0).await;
///     (StatusCode::CREATED, Json(user))
/// }
/// ```
#[proc_macro_attribute]
pub fn post(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("POST", attr, item)
}

/// Defines a PUT route handler with automatic dependency injection.
///
/// `Arc<T>` parameters are rewritten to `Inject<T>` (see [`get`] for details).
///
/// # Example
///
/// ```ignore
/// #[put("/users/{id}")]
/// async fn update_user(
///     path: Path<String>,
///     repo: Arc<UserRepo>,
///     body: Json<UpdateUser>,
/// ) -> impl IntoResponse {
///     Json(repo.update(&path, body.0).await)
/// }
/// ```
#[proc_macro_attribute]
pub fn put(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("PUT", attr, item)
}

/// Defines a DELETE route handler with automatic dependency injection.
///
/// `Arc<T>` parameters are rewritten to `Inject<T>` (see [`get`] for details).
///
/// # Example
///
/// ```ignore
/// #[delete("/users/{id}")]
/// async fn delete_user(
///     path: Path<String>,
///     repo: Arc<UserRepo>,
/// ) -> impl IntoResponse {
///     repo.delete(&path).await;
///     StatusCode::NO_CONTENT
/// }
/// ```
#[proc_macro_attribute]
pub fn delete(attr: TokenStream, item: TokenStream) -> TokenStream {
    route::generate_route("DELETE", attr, item)
}

/// Defines a PATCH route handler with automatic dependency injection.
///
/// `Arc<T>` parameters are rewritten to `Inject<T>` (see [`get`] for details).
///
/// # Example
///
/// ```ignore
/// #[patch("/users/{id}")]
/// async fn patch_user(
///     path: Path<String>,
///     repo: Arc<UserRepo>,
///     body: Json<PatchUser>,
/// ) -> impl IntoResponse {
///     Json(repo.patch(&path, body.0).await)
/// }
/// ```
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
/// Generates CRUD repository operations (create, update, find, delete) on `PgClient`.
///
/// # Struct Attributes
///
/// - `#[table("name")]` - Required. Database table name.
///
/// # Field Attributes
///
/// - `#[primary_key]` - Primary key field(s). Multiple fields create composite keys.
/// - `#[skip]` - Exclude from all DB operations. Field must implement `Default`.
/// - `#[skip_upsert]` - Exclude from UPDATE portion of upsert operations.
/// - `#[pg_type(Type)]` - Cast field to a different type when binding.
///
/// # Example
///
/// ```ignore
/// #[derive(PgEntity)]
/// #[table("users")]
/// pub struct User {
///     #[primary_key]
///     pub id: String,
///     pub name: String,
///     #[skip]
///     pub computed_field: String,
/// }
/// ```
#[proc_macro_derive(PgEntity, attributes(table, schema, primary_key, skip, skip_upsert, pg_type))]
pub fn pg_entity(input: TokenStream) -> TokenStream {
    pg_entity::generate_pg_entity(input)
}

/// Derive macro for generating REST CRUD endpoints with pagination.
///
/// Use alongside `PgEntity` to generate DTOs and route handlers
/// (GET, POST, PUT, PATCH, DELETE) registered via inventory.
///
/// # Struct Attributes
///
/// - `#[table("name")]` - Required. Database table name (from PgEntity).
/// - `#[crud(path = "/users")]` - REST path (default: pluralized snake_case).
/// - `#[crud(read_only)]` - Only generate read endpoints.
/// - `#[crud(skip_create)]` / `#[crud(skip_delete)]` - Skip specific endpoints.
///
/// # Field Attributes
///
/// - `#[primary_key]` - Primary key (from PgEntity).
/// - `#[auto_generated]` - DB-generated field (excluded from create/update DTOs).
/// - `#[readonly]` - Response-only field (e.g., `created_at`).
/// - `#[writeonly]` - Input-only field (e.g., `password_hash`).
///
/// # Example
///
/// ```ignore
/// #[derive(PgEntity, Crud)]
/// #[table("users")]
/// #[crud(path = "/users")]
/// pub struct User {
///     #[primary_key]
///     #[auto_generated]
///     pub id: Uuid,
///     pub name: String,
///     #[readonly]
///     pub created_at: DateTime<Utc>,
/// }
/// ```
#[proc_macro_derive(
    Crud,
    attributes(
        table,
        schema,
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
/// Replaces `main` with the Gearbox startup sequence (tokio runtime + framework init).
///
/// # Example
///
/// ```ignore
/// #[gearbox_app]
/// fn main() {}
/// ```
#[proc_macro_attribute]
pub fn gearbox_app(attr: TokenStream, item: TokenStream) -> TokenStream {
    app::generate_gearbox_app(attr, item)
}

/// Generate custom query methods on `PgClient`.
///
/// Define SQL queries with type-safe parameters and return types.
/// Placeholder count is validated at compile time.
///
/// # Return Types
///
/// - `Option<T>` — `fetch_optional`
/// - `Vec<T>` — `fetch_all`
/// - `T` (struct) — `fetch_one`
/// - Scalars (`i64`, `String`, etc.) — `query_scalar`
/// - `bool` — `rows_affected > 0`
/// - `u64` — `rows_affected`
/// - (none) — `execute`
///
/// # Example
///
/// ```ignore
/// pg_queries! {
///     fn find_by_email(email: &str) -> Option<User> {
///         "SELECT * FROM users WHERE email = $1"
///     }
///
///     fn count_active() -> i64 {
///         "SELECT COUNT(*) FROM users WHERE active = true"
///     }
/// }
/// ```
#[proc_macro]
pub fn pg_queries(input: TokenStream) -> TokenStream {
    pg_queries::pg_queries(input)
}
