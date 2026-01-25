mod client;
mod config;
pub mod crud;
mod entity;
mod error;
mod repository;

pub use client::PgClient;
pub use config::PgConfig;
pub use crud::QueryBuilder;
pub use entity::PgEntity;
pub use error::PgError;
pub use repository::PgRepository;

// Re-exports for macro use
pub use sqlx::postgres::PgRow;
pub use sqlx::query;
pub use sqlx::query_scalar;
pub use sqlx::Row;