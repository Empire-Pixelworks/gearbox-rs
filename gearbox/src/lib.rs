//! # Gearbox
//!
//! A lightweight Rust web framework with automatic dependency injection.
//!
//! ## Quick Start
//!
//! ```ignore
//! use gearbox_rs::prelude::*;
//!
//! #[cog]
//! pub struct MyService {
//!     // ...
//! }
//!
//! #[get("/hello")]
//! async fn hello() -> impl IntoResponse {
//!     "Hello, World!"
//! }
//!
//! #[gearbox_app]
//! fn main() {}
//! ```

// --- Public API (user-facing) ---
pub use gearbox_rs_core::{Cog, CogConfig, Config, Error, Gearbox, GearboxAppConfig, Hub, Inject};
#[cfg(feature = "testing")]
pub use gearbox_rs_core::TestHubBuilder;
pub use gearbox_rs_core::{IntoResponse, Json, Path, Query};
pub use gearbox_rs_core::crud;

// --- Doc-hidden (macro internals, must remain pub for macro expansion) ---
#[doc(hidden)]
pub use gearbox_rs_core::{
    async_trait, inventory, BoxFuture, CogFactory, CogRegistry, ConfigMeta, RouteRegistration,
    deserialize_config, validate_config,
};

// --- Macros (user-facing) ---
pub use gearbox_rs_macros::{cog, cog_config, delete, gearbox_app, get, patch, post, put};
#[cfg(feature = "postgres")]
pub use gearbox_rs_macros::{Crud, PgEntity, pg_queries};

// --- Postgres (conditional) ---
#[cfg(feature = "postgres")]
pub use gearbox_rs_postgres;
#[cfg(feature = "postgres")]
pub use gearbox_rs_postgres::{PgClient, PgConfig, PgError, PgEntity, PgRepository, SchemaConfig};
#[cfg(feature = "postgres")]
#[doc(hidden)]
pub use gearbox_rs_postgres::{PgRow, Row, query, query_scalar};

/// Convenience prelude that re-exports commonly used items.
pub mod prelude {
    // Framework core
    pub use gearbox_rs_core::{Cog, CogConfig, Config, Error, Hub};

    // Axum re-exports
    pub use gearbox_rs_core::{Inject, IntoResponse, Json, Path, Query};

    // Macros
    pub use gearbox_rs_macros::{cog, cog_config, delete, gearbox_app, get, patch, post, put};

    // Testing (conditional)
    #[cfg(feature = "testing")]
    pub use gearbox_rs_core::TestHubBuilder;

    // Postgres (conditional)
    #[cfg(feature = "postgres")]
    pub use gearbox_rs_macros::{Crud, PgEntity, pg_queries};
    #[cfg(feature = "postgres")]
    pub use gearbox_rs_postgres::{PgClient, PgConfig, PgError, PgRepository, SchemaConfig};
}
