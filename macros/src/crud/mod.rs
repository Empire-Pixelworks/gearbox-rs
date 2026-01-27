//! CRUD derive macro implementation.
//!
//! This module generates REST CRUD endpoints with filtering, pagination,
//! and optional OpenAPI documentation via utoipa.

mod dto;
mod openapi;
mod parse;
mod routes;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

use parse::parse_crud_entity;

/// Main entry point for the Crud derive macro.
pub fn generate_crud(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let entity = parse_crud_entity(&input);

    let dto_tokens = dto::generate_dtos(&entity);

    let where_clause_tokens = dto::generate_where_clause_impl(&entity);

    let route_tokens = routes::generate_routes(&entity);

    let openapi_tokens = openapi::generate_openapi(&entity);

    let expanded = quote! {
        #dto_tokens
        #where_clause_tokens
        #route_tokens
        #openapi_tokens
    };

    expanded.into()
}
