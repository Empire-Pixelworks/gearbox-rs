use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::crud::parse::CrudEntityInfo;

/// Generate the partial update handler (PATCH /{path}/{id}).
pub(super) fn generate_patch_handler(entity: &CrudEntityInfo, path: &str, core: &TokenStream2, pg: &TokenStream2) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_patch", snake_name);
    let update_name = format_ident!("{}Update", name);
    let response_name = format_ident!("{}Response", name);
    let id_path = format!("{}/{{id}}", path);

    let pk_fields = entity.pk_fields();
    let id_type = super::pk_id_type(&pk_fields);

    // Generate field update logic
    let update_fields = entity.update_fields();
    let field_updates: Vec<TokenStream2> = update_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            let ty_str = quote!(#ty).to_string();
            if ty_str.starts_with("Option") {
                // If the field is already Option, replace if Some
                quote! {
                    if body.#ident.is_some() {
                        existing.#ident = body.#ident;
                    }
                }
            } else {
                // Otherwise unwrap the Option from Update DTO
                quote! {
                    if let Some(val) = body.#ident {
                        existing.#ident = val;
                    }
                }
            }
        })
        .collect();

    let not_found = super::not_found_response(core);
    let fetch_err = super::internal_error_response(core, &snake_name, "fetch");
    let update_err = super::internal_error_response(core, &snake_name, "update");

    let handler = quote! {
        async fn #handler_name(
            #core::Inject(db): #core::Inject<#pg::PgClient>,
            #core::Path(id): #core::Path<#id_type>,
            #core::Json(body): #core::Json<#update_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            // First, fetch the existing entity
            let find_result: Result<Option<#name>, #pg::PgError> =
                <#pg::PgClient as #pg::PgRepository<#name>>::find_by_id(&*db, &id).await;

            let mut existing = match find_result {
                Ok(Some(entity)) => entity,
                Ok(None) => {
                    return #not_found;
                }
                Err(e) => {
                    return #fetch_err;
                }
            };

            // Apply partial updates
            #(#field_updates)*

            // Save the updated entity
            let update_result: Result<#name, #pg::PgError> =
                <#pg::PgClient as #pg::PgRepository<#name>>::update(&*db, existing).await;

            match update_result {
                Ok(updated) => {
                    let response = #response_name::from(updated);
                    (axum::http::StatusCode::OK, #core::Json(response)).into_response()
                }
                Err(e) => {
                    #update_err
                }
            }
        }
    };

    let registration = quote! {
        #core::inventory::submit!(#core::RouteRegistration {
            path: #id_path,
            method: "PATCH",
            handler: || axum::routing::patch(#handler_name),
        });
    };

    (handler, registration)
}
