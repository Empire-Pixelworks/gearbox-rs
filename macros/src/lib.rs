mod cog;
mod config;
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
/// use gearbox_macros::cog;
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
