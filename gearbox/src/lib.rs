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

pub use gearbox_rs_core::*;
pub use gearbox_rs_macros::*;

#[cfg(feature = "postgres")]
pub use gearbox_rs_postgres;

/// Convenience prelude that re-exports commonly used items.
pub mod prelude {
    pub use gearbox_rs_core::{
        Cog, CogConfig, Config, Error, Hub, Inject, IntoResponse, Json, Path, Query,
    };
    pub use gearbox_rs_macros::{cog, cog_config, delete, gearbox_app, get, patch, post, put};

    #[cfg(feature = "postgres")]
    pub use gearbox_rs_macros::{PgEntity, pg_queries};

    #[cfg(feature = "postgres")]
    pub use gearbox_rs_postgres::{PgClient, PgConfig, PgError};
}
