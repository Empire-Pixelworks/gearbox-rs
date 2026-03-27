use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::crud::parse::CrudEntityInfo;

/// Generate the full update handler (PUT /{path}/{id}).
pub(super) fn generate_put_handler(entity: &CrudEntityInfo, path: &str, core: &TokenStream2, pg: &TokenStream2) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_update", snake_name);
    let create_name = format_ident!("{}Create", name);
    let response_name = format_ident!("{}Response", name);
    let id_path = format!("{}/{{id}}", path);

    let pk_fields = entity.pk_fields();
    let id_type = super::pk_id_type(&pk_fields);

    // For PUT, we use the Create DTO but replace auto-generated fields with the provided ID
    let auto_gen_fields: Vec<_> = entity
        .fields
        .iter()
        .filter(|f| f.auto_generated && !f.skip)
        .collect();

    let entity_creation = if auto_gen_fields.is_empty() {
        quote! {
            let entity = body.into_entity();
        }
    } else if auto_gen_fields.len() == 1 && auto_gen_fields[0].is_primary_key {
        // Single auto-generated primary key - use the ID from path
        quote! {
            let entity = body.into_entity(id.clone());
        }
    } else {
        // Multiple auto-generated fields - need more complex handling
        let auto_gen_values: Vec<TokenStream2> = auto_gen_fields
            .iter()
            .map(|f| {
                if f.is_primary_key {
                    quote! { id.clone() }
                } else {
                    let ty = &f.ty;
                    if super::type_is_uuid(ty) {
                        quote! { uuid::Uuid::new_v4() }
                    } else {
                        quote! { Default::default() }
                    }
                }
            })
            .collect();
        quote! {
            let entity = body.into_entity(#(#auto_gen_values),*);
        }
    };

    let not_found = super::not_found_response(core);
    let err_response = super::internal_error_response(core, &snake_name, "update");

    let handler = quote! {
        async fn #handler_name(
            #core::Inject(db): #core::Inject<#pg::PgClient>,
            #core::Path(id): #core::Path<#id_type>,
            #core::Json(body): #core::Json<#create_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #entity_creation

            let result: Result<#name, #pg::PgError> =
                <#pg::PgClient as #pg::PgRepository<#name>>::update(&*db, entity).await;

            match result {
                Ok(updated) => {
                    let response = #response_name::from(updated);
                    (axum::http::StatusCode::OK, #core::Json(response)).into_response()
                }
                Err(#pg::PgError::NotFound) => {
                    #not_found
                }
                Err(e) => {
                    #err_response
                }
            }
        }
    };

    let registration = quote! {
        #core::inventory::submit!(#core::RouteRegistration {
            path: #id_path,
            method: "PUT",
            handler: || axum::routing::put(#handler_name),
        });
    };

    (handler, registration)
}
