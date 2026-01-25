//! OpenAPI documentation generation for CRUD operations.
//!
//! This module generates utoipa annotations for the generated DTOs and routes
//! when the `openapi` feature is enabled.

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::parse::CrudEntityInfo;

/// Generate OpenAPI annotations for the entity.
///
/// This generates `#[utoipa::path(...)]` annotations for each route handler.
/// The actual derive attributes for DTOs are added inline in dto.rs using `cfg_attr`.
pub fn generate_openapi(_entity: &CrudEntityInfo) -> TokenStream2 {
    // OpenAPI path annotations are feature-gated
    // The actual utoipa derives on structs are handled via cfg_attr in dto.rs

    // For now, we don't generate path annotations since they require
    // more complex integration with utoipa's OpenApi derive.
    // The DTOs already have the necessary derives via cfg_attr.

    // In a full implementation, you would generate something like:
    // #[cfg(feature = "openapi")]
    // impl utoipa::OpenApiSchema for UserResponse { ... }

    // For now, return empty - the schema generation comes from the derives
    quote! {}
}

/// Helper to generate OpenAPI operation ID from handler name.
#[allow(dead_code)]
fn operation_id(entity_name: &str, operation: &str) -> String {
    format!("{}_{}", operation, entity_name.to_lowercase())
}

/// Generate OpenAPI tags for the entity.
#[allow(dead_code)]
fn generate_tags(entity: &CrudEntityInfo) -> Vec<String> {
    vec![entity.name.to_string()]
}
