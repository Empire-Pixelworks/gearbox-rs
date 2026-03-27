//! CRUD derive macro implementation.
//!
//! This module generates REST CRUD endpoints with pagination.

mod dto;
mod parse;
mod routes;
mod utils;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, parse_macro_input};

use parse::parse_crud_entity;

pub fn generate_crud(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate_crud_internal(input).unwrap_or_else(|e| e.to_compile_error().into())
}

pub fn generate_crud_internal(input: DeriveInput) -> Result<TokenStream, Error> {
    let entity = parse_crud_entity(&input)?;
    let dto_tokens = dto::generate_dtos(&entity);
    let route_tokens = routes::generate_routes(&entity);

    let expanded = quote! {
        #dto_tokens
        #route_tokens
    };

    Ok(expanded.into())
}
