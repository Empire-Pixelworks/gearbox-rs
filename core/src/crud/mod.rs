//! CRUD support traits and types.
//!
//! This module provides runtime support for the `#[derive(Crud)]` macro,
//! including pagination and response wrapping.

mod response;

pub use response::PagedResponse;
