//! CRUD support traits and types.
//!
//! This module provides runtime support for the `#[derive(Crud)]` macro,
//! including query building, pagination, and response wrapping.

mod query;
mod response;

pub use query::{BuildWhereClause, SortDirection, SortSpec};
pub use response::PagedResponse;
