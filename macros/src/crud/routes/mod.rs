//! Route handler generation for CRUD operations.

mod create;
mod delete;
mod get;
mod list;
mod patch;
mod update;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use crate::crud::parse::{CrudEntityInfo, CrudFieldInfo};

/// Default pagination limit for list endpoints.
pub(super) const DEFAULT_PAGE_LIMIT: i64 = 100;

/// Determine the path-parameter type for primary key(s).
pub(super) fn pk_id_type(pk_fields: &[&CrudFieldInfo]) -> TokenStream2 {
    if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        quote! { String }
    }
}

/// Check if a syn::Type's last path segment is "Uuid".
pub(super) fn type_is_uuid(ty: &syn::Type) -> bool {
    if let syn::Type::Path(tp) = ty {
        tp.path.segments.last().is_some_and(|seg| seg.ident == "Uuid")
    } else {
        false
    }
}

/// Generate TokenStream for a 500 Internal Server Error response.
///
/// The returned fragment references an unresolved ident `e`, which must be
/// bound by the caller's `Err(e)` pattern.
pub(super) fn internal_error_response(
    core: &TokenStream2,
    entity_name: &str,
    operation: &str,
) -> TokenStream2 {
    quote! {
        (axum::http::StatusCode::INTERNAL_SERVER_ERROR,
         #core::Json(#core::crud::ErrorResponse::new(
             format!("Failed to {} {}: {}", #operation, #entity_name, e)
         ))).into_response()
    }
}

/// Generate TokenStream for a 404 Not Found response.
pub(super) fn not_found_response(core: &TokenStream2) -> TokenStream2 {
    quote! {
        (axum::http::StatusCode::NOT_FOUND,
         #core::Json(#core::crud::ErrorResponse::new("Not found"))).into_response()
    }
}

/// Generate all route handlers for the entity.
pub fn generate_routes(entity: &CrudEntityInfo) -> TokenStream2 {
    let core = crate::paths::core_crate();
    let pg = crate::paths::pg_crate();
    let path = entity.effective_path();

    // Generate handlers based on config
    let mut handlers = Vec::new();
    let mut registrations = Vec::new();

    // GET list - always generated
    let (list_handler, list_reg) = list::generate_list_handler(entity, &path, &core, &pg);
    handlers.push(list_handler);
    registrations.push(list_reg);

    // GET by id - always generated
    let (get_handler, get_reg) = get::generate_get_handler(entity, &path, &core, &pg);
    handlers.push(get_handler);
    registrations.push(get_reg);

    // POST create - unless read_only or skip_create
    if !entity.config.read_only && !entity.config.skip_create {
        let (create_handler, create_reg) = create::generate_create_handler(entity, &path, &core, &pg);
        handlers.push(create_handler);
        registrations.push(create_reg);
    }

    // PUT update (full) - unless read_only
    if !entity.config.read_only {
        let (put_handler, put_reg) = update::generate_put_handler(entity, &path, &core, &pg);
        handlers.push(put_handler);
        registrations.push(put_reg);
    }

    // PATCH update (partial) - unless read_only
    if !entity.config.read_only {
        let (patch_handler, patch_reg) = patch::generate_patch_handler(entity, &path, &core, &pg);
        handlers.push(patch_handler);
        registrations.push(patch_reg);
    }

    // DELETE - unless read_only or skip_delete
    if !entity.config.read_only && !entity.config.skip_delete {
        let (delete_handler, delete_reg) = delete::generate_delete_handler(entity, &path, &core, &pg);
        handlers.push(delete_handler);
        registrations.push(delete_reg);
    }

    quote! {
        #(#handlers)*
        #(#registrations)*
    }
}
