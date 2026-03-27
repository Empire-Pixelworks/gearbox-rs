# Gearbox

A lightweight, opinionated Rust web framework with automatic dependency injection.

## Features

- **Dependency Injection** - Declare dependencies with `#[inject]` and let Gearbox wire everything together
- **Auto-Registration** - Services (Cogs) are automatically discovered and registered at startup
- **Dependency Resolution** - Topological sorting ensures services are initialized in the correct order
- **Configuration System** - Load config from TOML files and environment variables with relaxed binding
- **Route Macros** - Define HTTP handlers with `#[get]`, `#[post]`, etc. and automatic parameter injection
- **PostgreSQL Support** - `#[derive(PgEntity)]` generates CRUD operations, `pg_queries!` for custom SQL
- **REST API Generation** - `#[derive(Crud)]` generates complete REST endpoints with pagination
- **Built on Axum** - Leverages the battle-tested Axum web framework under the hood

## Quick Start

Add Gearbox to your `Cargo.toml`:

```toml
[dependencies]
gearbox-rs = "0.1"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }

# For PostgreSQL support:
# gearbox-rs = { version = "0.1", features = ["postgres"] }
```

## Hello World Example

```rust
use gearbox_rs::prelude::*;
use std::sync::Arc;

// Helper function for default message
fn default_message() -> String {
    "Hello".to_string()
}

// Define a service (Cog)
#[cog]
pub struct Greeter {
    #[default(default_message)]
    message: String,
}

impl Greeter {
    pub fn greet(&self, name: &str) -> String {
        format!("{}, {}!", self.message, name)
    }
}

// Define a route that uses the service
#[get("/hello/{name}")]
async fn hello(
    path: Path<String>,
    greeter: Arc<Greeter>,
) -> impl IntoResponse {
    greeter.greet(&path)
}

// Start the application
#[gearbox_app]
fn main() {}
```

Run with:
```bash
cargo run
```

Visit `http://localhost:8080/hello/world` to see: `Hello, world!`

## Core Concepts

### Cogs (Services)

A **Cog** is Gearbox's term for a service or component. Use the `#[cog]` macro to define one:

```rust
#[cog]
pub struct UserService {
    #[inject]
    db: Arc<Database>,       // Injected from registry
    #[config]
    settings: UserConfig,    // Loaded from configuration
    #[default(Vec::new)]
    cache: Vec<User>,        // Initialized via function
    counter: u64,            // Uses Default::default()
}
```

Field attributes:
- `#[inject]` - Inject an `Arc<T>` from the service registry
- `#[config]` - Load from configuration (requires `CogConfig` impl)
- `#[default(fn)]` - Initialize via a sync function
- `#[default_async(fn)]` - Initialize via an async function
- No attribute - Uses `Default::default()`

### Configuration

Define configuration structs with `#[cog_config]`:

```rust
#[cog_config("database")]
#[derive(Default, Deserialize)]
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
}
```

Create a `config.toml`:

```toml
[gearbox_app]
http-port = 8080
log-level = "info"

[database]
url = "postgres://localhost/mydb"
max-connections = 10
```

Override with environment variables:

```bash
GEARBOX_DATABASE_URL=postgres://prod/mydb
GEARBOX_DATABASE_MAXCONNECTIONS=50
GEARBOX_GEARBOXAPP_HTTPPORT=3000
```

Gearbox uses **relaxed binding** - all of these match `max_connections`:
- `max-connections` (TOML kebab-case)
- `max_connections` (TOML snake_case)
- `MAXCONNECTIONS` (env var)

#### Default Values

If a TOML section is missing entirely, Gearbox uses `Default::default()` for that config type. For partial sections (some fields present, others missing), use `#[serde(default)]` on the struct to fill in missing fields from `Default`:

```rust
#[cog_config("database")]
#[derive(Default, Clone, Deserialize)]
#[serde(default)]  // Missing fields use Default impl values
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
}

impl Default for DbConfig {
    fn default() -> Self {
        Self {
            url: String::new(),
            max_connections: 10,
        }
    }
}
```

#### Config Validation

Add a validation function and reference it in the macro to catch invalid values at startup:

```rust
fn validate_db(config: &DbConfig) -> Result<(), String> {
    if config.url.is_empty() {
        return Err("database url must not be empty".to_string());
    }
    if config.max_connections == 0 {
        return Err("max_connections must be > 0".to_string());
    }
    Ok(())
}

#[cog_config("database", validate = "validate_db")]
#[derive(Default, Clone, Deserialize)]
#[serde(default)]
pub struct DbConfig {
    pub url: String,
    pub max_connections: u32,
}
```

Validation runs automatically after deserialization during `Gearbox::crank()`. If validation fails, startup aborts with a clear error message.

#### Startup Diagnostics

Gearbox logs configuration status at startup:
- **WARN** for each config section not found in the file/env (falling back to defaults)
- **WARN** for TOML sections that exist but have no matching `#[cog_config]` type (possible typos)

### Routes

Define HTTP handlers with route macros:

```rust
#[get("/users")]
async fn list_users(repo: Arc<UserRepo>) -> Json<Vec<User>> {
    Json(repo.find_all().await)
}

#[post("/users")]
async fn create_user(
    repo: Arc<UserRepo>,
    body: Json<CreateUser>,
) -> impl IntoResponse {
    let user = repo.create(body.0).await;
    (StatusCode::CREATED, Json(user))
}

#[get("/users/{id}")]
async fn get_user(
    path: Path<String>,
    repo: Arc<UserRepo>,
) -> impl IntoResponse {
    match repo.find_by_id(&path).await {
        Some(user) => Json(user).into_response(),
        None => StatusCode::NOT_FOUND.into_response(),
    }
}
```

Available macros: `#[get]`, `#[post]`, `#[put]`, `#[delete]`, `#[patch]`

`Arc<T>` parameters are automatically transformed to use Gearbox's `Inject<T>` extractor. Other Axum extractors (`Json`, `Path`, `Query`, etc.) work as normal.

#### How Parameter Injection Works

When you write a handler like this:

```rust
#[get("/users")]
async fn list(repo: Arc<UserRepo>) -> Json<Vec<User>> { ... }
```

The route macro rewrites it (roughly) to:

```rust
async fn __handler_list(Inject(repo): Inject<UserRepo>) -> Json<Vec<User>> { ... }
```

`Inject<T>` is an axum extractor that implements `FromRequestParts`. At request time it:
1. Extracts the `Arc<Hub>` from axum's request state.
2. Calls `hub.registry.get::<T>()` to look up the Cog by type.
3. Returns the `Arc<T>` wrapped in `Inject(...)`.

Non-`Arc` parameters (`Json<T>`, `Path<T>`, `Query<T>`, etc.) are passed through to axum unchanged. This means you can mix injected services with standard extractors freely.

### PostgreSQL Entities

Generate repository operations with `#[derive(PgEntity)]`:

```rust
#[derive(PgEntity)]
#[table("users")]
pub struct User {
    #[primary_key]
    pub id: String,
    pub name: String,
    pub email: String,
    #[skip]
    pub computed: String,  // Excluded from DB operations
}

// Generated methods on PgClient:
// - create(entity) -> INSERT
// - update(entity) -> UPDATE
// - upsert(entity) -> INSERT ... ON CONFLICT
// - find_by_id(id) -> SELECT
// - find_by_ids(ids) -> SELECT ... IN
// - find_page(limit, offset) -> SELECT with pagination
// - exists(id) -> SELECT EXISTS
// - count() -> SELECT COUNT
// - delete(id) -> DELETE
// - create_batch(entities) -> batch INSERT
// - upsert_batch(entities) -> batch UPSERT
```

### Custom Queries

Define custom SQL queries with `pg_queries!`:

```rust
use gearbox_rs::prelude::pg_queries;

#[derive(sqlx::FromRow)]
pub struct UserSummary {
    pub id: String,
    pub name: String,
}

pg_queries! {
    fn find_user_by_email(email: &str) -> Option<User> {
        "SELECT * FROM users WHERE email = $1"
    }

    fn find_active_users() -> Vec<User> {
        "SELECT * FROM users WHERE active = true"
    }

    fn get_user_summary(id: &str) -> Option<UserSummary> {
        "SELECT id, name FROM users WHERE id = $1"
    }

    fn count_users_by_role(role: &str) -> i64 {
        "SELECT COUNT(*) FROM users WHERE role = $1"
    }

    fn deactivate_user(id: &str) -> bool {
        "UPDATE users SET active = false WHERE id = $1"
    }

    fn delete_inactive() -> u64 {
        "DELETE FROM users WHERE active = false"
    }

    fn log_action(user_id: &str, action: &str) {
        "INSERT INTO audit_log (user_id, action) VALUES ($1, $2)"
    }
}
```

When using multi-schema support, declare the schema at the top of the block:

```rust
pg_queries! {
    schema = "accounts";

    fn find_user_by_email(email: &str) -> Option<User> {
        "SELECT * FROM users WHERE email = $1"
    }
}
```

Usage - import the generated `PgQueries` trait (or `PgQueries{Schema}` when using a named schema):

```rust
use crate::PgQueries;

let user = client.find_user_by_email("test@example.com").await?;
let count = client.count_users_by_role("admin").await?;
let deleted = client.deactivate_user("123").await?; // returns bool
```

Return type mapping:

| Return Type | Behavior |
|-------------|----------|
| `Option<T>` | `fetch_optional` - returns `None` if no row |
| `Vec<T>` | `fetch_all` - returns all matching rows |
| `T` (struct) | `fetch_one` - errors if no row found |
| `i64`, `String`, etc. | `query_scalar` - single column value |
| `bool` | `execute` - `true` if rows_affected > 0 |
| `u64` | `execute` - returns rows_affected |
| (none) | `execute` - returns `()` |

#### Complex Joins

Use custom structs to represent JOIN results:

```rust
// Define a struct for the join result
#[derive(sqlx::FromRow)]
pub struct OrderWithCustomer {
    // Order fields
    pub order_id: i64,
    pub order_date: chrono::NaiveDate,
    pub total: f64,
    // Customer fields from JOIN
    pub customer_name: String,
    pub customer_email: String,
}

#[derive(sqlx::FromRow)]
pub struct ProductSalesReport {
    pub product_name: String,
    pub category: String,
    pub total_sold: i64,
    pub revenue: f64,
}

pg_queries! {
    fn find_orders_with_customers(status: &str) -> Vec<OrderWithCustomer> {
        "SELECT
            o.id as order_id,
            o.order_date,
            o.total,
            c.name as customer_name,
            c.email as customer_email
         FROM orders o
         JOIN customers c ON o.customer_id = c.id
         WHERE o.status = $1
         ORDER BY o.order_date DESC"
    }

    fn get_sales_report(start_date: chrono::NaiveDate, end_date: chrono::NaiveDate) -> Vec<ProductSalesReport> {
        "SELECT
            p.name as product_name,
            cat.name as category,
            SUM(oi.quantity) as total_sold,
            SUM(oi.quantity * oi.unit_price) as revenue
         FROM order_items oi
         JOIN products p ON oi.product_id = p.id
         JOIN categories cat ON p.category_id = cat.id
         JOIN orders o ON oi.order_id = o.id
         WHERE o.order_date BETWEEN $1 AND $2
         GROUP BY p.id, p.name, cat.name
         ORDER BY revenue DESC"
    }

    fn find_user_with_latest_order(user_id: &str) -> Option<UserWithOrder> {
        "SELECT
            u.id, u.name, u.email,
            o.id as last_order_id,
            o.total as last_order_total,
            o.order_date as last_order_date
         FROM users u
         LEFT JOIN LATERAL (
            SELECT id, total, order_date
            FROM orders
            WHERE user_id = u.id
            ORDER BY order_date DESC
            LIMIT 1
         ) o ON true
         WHERE u.id = $1"
    }
}
```

The struct field names must match the column aliases in your SQL query. Use `AS` to rename columns from joins to avoid conflicts and match your struct fields.

### REST API Generation with Crud

Generate complete REST endpoints with `#[derive(Crud)]`:

```rust
#[derive(PgEntity, Crud)]
#[table("users")]
#[crud(path = "/users")]
pub struct User {
    #[primary_key]
    #[auto_generated]
    pub id: Uuid,

    pub name: String,
    pub email: String,

    #[writeonly]
    pub password_hash: String,

    #[readonly]
    pub created_at: DateTime<Utc>,

    pub updated_at: DateTime<Utc>,
    pub active: bool,
}
```

This generates:

**DTOs:**
- `UserCreate` - For POST requests (excludes `id`, `created_at`)
- `UserUpdate` - For PATCH requests (all fields optional)
- `UserQuery` - Query parameters for pagination
- `UserResponse` - For responses (excludes `password_hash`)

**Routes:**
| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List with pagination |
| GET | `/users/{id}` | Get single entity |
| POST | `/users` | Create new entity |
| PUT | `/users/{id}` | Full update |
| PATCH | `/users/{id}` | Partial update |
| DELETE | `/users/{id}` | Delete entity |

#### Field Attributes

| Attribute | Description |
|-----------|-------------|
| `#[auto_generated]` | DB generates this (UUID, serial). Excluded from create/update DTOs |
| `#[readonly]` | Only in responses (e.g., `created_at`). Excluded from create/update |
| `#[writeonly]` | Only in create/update (e.g., `password_hash`). Excluded from responses |

#### Query Examples

```bash
# Paginate results
GET /users?limit=10&offset=0
```

For custom filtering, use `pg_queries!` with a custom route handler.

#### Struct-Level Options

```rust
#[derive(PgEntity, Crud)]
#[table("audit_logs")]
#[crud(path = "/logs", read_only)]  // Only GET endpoints
pub struct AuditLog { ... }

#[crud(skip_create)]   // No POST endpoint
#[crud(skip_delete)]   // No DELETE endpoint
```

### Multi-Schema Support

Gearbox supports multiple PostgreSQL schemas within a single database instance. This is useful for consolidating multiple apps into a single monolith while keeping their data isolated.

#### Configuration

Define multiple schemas in `config.toml`:

```toml
[postgres]
database_url = "localhost:5432"
database_username = "postgres"
database_password = "postgres"
max_connections = 5

[postgres.schemas.accounts]
schema_name = "accounts"
migration_path = "./migrations/accounts"

[postgres.schemas.billing]
schema_name = "billing"
migration_path = "./migrations/billing"
```

Each schema gets its own connection pool with `search_path` pinned, and its own migration directory. Schemas are created automatically on startup.

#### Entity Schema Binding

Annotate entities with `#[schema("name")]` to bind them to a specific schema's pool:

```rust
#[derive(PgEntity, Crud)]
#[table("users")]
#[schema("accounts")]
#[crud(path = "/users")]
pub struct User {
    #[primary_key]
    pub id: Uuid,
    pub name: String,
}

#[derive(PgEntity, Crud)]
#[table("invoices")]
#[schema("billing")]
#[crud(path = "/invoices")]
pub struct Invoice {
    #[primary_key]
    pub id: Uuid,
    pub amount: f64,
}
```

Each entity's generated repository methods automatically route to the correct pool. Cross-schema queries are discouraged by design — each entity is bound to exactly one schema.

#### Custom Queries with Schemas

Each `pg_queries!` block declares which schema it targets:

```rust
pg_queries! {
    schema = "accounts";

    fn find_user_by_email(email: &str) -> Option<User> {
        "SELECT * FROM users WHERE email = $1"
    }
}

pg_queries! {
    schema = "billing";

    fn get_unpaid_invoices() -> Vec<Invoice> {
        "SELECT * FROM invoices WHERE paid = false"
    }
}
```

#### Backward Compatibility

The legacy single-schema config format still works:

```toml
[postgres]
schema_name = "my_app"
migration_path = "./migrations"
```

Entities without a `#[schema("...")]` attribute use the `"default"` pool, which maps to the legacy config. Existing apps require zero changes.

## Middleware

Gearbox exposes Axum's middleware system through the `router_with` method. Use it to add tower layers for CORS, tracing, compression, auth, and more.

### Adding Middleware

When you need middleware, write `main` manually instead of using `#[gearbox_app]`:

```rust
use gearbox_rs::{Error, Gearbox};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

#[tokio::main]
async fn main() -> Result<(), Error> {
    Gearbox::crank().await?
        .router_with(|router| {
            router
                .layer(CorsLayer::permissive())
                .layer(TraceLayer::new_for_http())
        })
        .ignite()
        .await
}
```

The closure receives the fully-built `axum::Router` (after state is applied), so any tower `Layer` works directly. `#[gearbox_app]` still works for apps that don't need middleware.

### Lifecycle Hooks

Cogs can opt in to lifecycle callbacks with `#[on_start]` and `#[on_shutdown]` attributes:

```rust
#[cog]
#[on_start(initialize)]
#[on_shutdown(cleanup)]
pub struct MyService {
    #[inject]
    db: Arc<Database>,
}

impl MyService {
    async fn initialize(&self) -> Result<(), Error> {
        tracing::info!("MyService started, warming cache...");
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), Error> {
        tracing::info!("MyService shutting down, flushing buffers...");
        Ok(())
    }
}
```

- `on_start` is called after all cogs are initialized, in dependency order
- `on_shutdown` is called when the server receives Ctrl+C or SIGTERM, in reverse dependency order
- Both are optional - cogs without these attributes have no-op defaults

See `examples/03-middleware/` for a complete working example.

### Dependency Resolution

Gearbox uses topological sorting (Kahn's algorithm) to determine the order in which Cogs are initialized. Each `#[inject]` field declares a dependency, and Cogs with no dependencies are built first.

```
Database (no deps)           -> initialized first
  |
UserRepo (#[inject] Database)  -> initialized second
  |
UserService (#[inject] UserRepo) -> initialized third
```

**Cyclic dependencies** are detected at startup. If Cog A depends on B and B depends on A, `Gearbox::crank()` returns an error:

```
Error: Cyclic dependency detected involving: CogA, CogB
```

To resolve a cycle, restructure your services to break the circular reference — for example, extract shared logic into a third Cog.

**Missing dependencies** are also caught at startup. If a `#[inject]` field references a type that has no corresponding `#[cog]` struct, you'll see:

```
Error: Missing dependency: 'UserService' requires '<TypeId>' which is not registered
```

Common causes: forgetting to annotate a struct with `#[cog]`, or not including the crate that defines it.

**Shutdown order** is the reverse of initialization order — dependents shut down before their dependencies. Each Cog's `on_shutdown` hook has a 30-second timeout; timeouts are logged but do not block other Cogs from shutting down.

## Project Structure

```
gearbox-rs/
├── core/           # Core framework (Hub, Config, DI, routing)
├── macros/         # Procedural macros (#[cog], #[get], etc.)
├── pg/             # PostgreSQL integration
└── examples/       # Example applications
```

## License

MIT
