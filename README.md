# Gearbox

A lightweight, opinionated Rust web framework with automatic dependency injection.

## Features

- **Dependency Injection** - Declare dependencies with `#[inject]` and let Gearbox wire everything together
- **Auto-Registration** - Services (Cogs) are automatically discovered and registered at startup
- **Dependency Resolution** - Topological sorting ensures services are initialized in the correct order
- **Configuration System** - Load config from TOML files and environment variables with relaxed binding
- **Route Macros** - Define HTTP handlers with `#[get]`, `#[post]`, etc. and automatic parameter injection
- **PostgreSQL Support** - `#[derive(PgEntity)]` generates CRUD operations, `pg_queries!` for custom SQL
- **REST API Generation** - `#[derive(Crud)]` generates complete REST endpoints with filtering and pagination
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
#[get("/hello/:name")]
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
[gearbox-app]
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

#[get("/users/:id")]
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

Usage - import the `PgQueries` trait:

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

    #[searchable]
    pub name: String,

    #[searchable(eq)]
    pub email: String,

    #[writeonly]
    pub password_hash: String,

    #[readonly]
    pub created_at: DateTime<Utc>,

    #[searchable]
    pub updated_at: DateTime<Utc>,

    pub active: bool,
}
```

This generates:

**DTOs:**
- `UserCreate` - For POST requests (excludes `id`, `created_at`)
- `UserUpdate` - For PATCH requests (all fields optional)
- `UserQuery` - Query parameters with filtering operators
- `UserResponse` - For responses (excludes `password_hash`)

**Routes:**
| Method | Path | Description |
|--------|------|-------------|
| GET | `/users` | List with filtering and pagination |
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
| `#[searchable]` | Include in query params with type-based operators |
| `#[searchable(eq, like)]` | Override default operators |

#### Searchable Operators by Type

| Type | Default Operators |
|------|-------------------|
| `String` | `eq`, `like` (ILIKE), `starts_with` |
| `i32`, `i64`, `f64` | `eq`, `gt`, `gte`, `lt`, `lte` |
| `DateTime`, `NaiveDate` | `eq`, `gt`, `gte`, `lt`, `lte` |
| `bool`, `Uuid` | `eq` |

#### Query Examples

```bash
# Filter by name containing "john" (case-insensitive)
GET /users?name_like=john

# Filter by exact email
GET /users?email=john@example.com

# Filter by date range with sorting and pagination
GET /users?updated_at_gte=2024-01-01&sort=-created_at&limit=10&offset=0
```

#### Struct-Level Options

```rust
#[derive(PgEntity, Crud)]
#[table("audit_logs")]
#[crud(path = "/logs", read_only)]  // Only GET endpoints
pub struct AuditLog { ... }

#[crud(skip_create)]   // No POST endpoint
#[crud(skip_delete)]   // No DELETE endpoint
```

#### OpenAPI Support

Enable the `openapi` feature for automatic utoipa schema generation:

```toml
gearbox-rs = { version = "0.1", features = ["postgres", "openapi"] }
```

The generated DTOs will include `#[derive(utoipa::ToSchema)]` and query DTOs will include `#[derive(utoipa::IntoParams)]`.

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
