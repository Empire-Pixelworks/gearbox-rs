# Gearbox

A lightweight, opinionated Rust web framework with automatic dependency injection.

## Features

- **Dependency Injection** - Declare dependencies with `#[inject]` and let Gearbox wire everything together
- **Auto-Registration** - Services (Cogs) are automatically discovered and registered at startup
- **Dependency Resolution** - Topological sorting ensures services are initialized in the correct order
- **Configuration System** - Load config from TOML files and environment variables with relaxed binding
- **Route Macros** - Define HTTP handlers with `#[get]`, `#[post]`, etc. and automatic parameter injection
- **PostgreSQL Support** - `#[derive(PgEntity)]` generates CRUD operations for your entities
- **Built on Axum** - Leverages the battle-tested Axum web framework under the hood

## Quick Start

Add Gearbox to your `Cargo.toml`:

```toml
[dependencies]
gearbox-core = { path = "../core" }
gearbox-macros = { path = "../macros" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Hello World Example

```rust
use gearbox_core::{IntoResponse, Path};
use gearbox_macros::{cog, gearbox_app, get};
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
