use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::crud::parse::CrudEntityInfo;

/// Generate the delete handler (DELETE /{path}/{id}).
pub(super) fn generate_delete_handler(entity: &CrudEntityInfo, path: &str, core: &TokenStream2, pg: &TokenStream2) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_delete", snake_name);
    let id_path = format!("{}/{{id}}", path);

    let pk_fields = entity.pk_fields();
    let id_type = super::pk_id_type(&pk_fields);

    let not_found = super::not_found_response(core);
    let err_response = super::internal_error_response(core, &snake_name, "delete");

    let handler = quote! {
        async fn #handler_name(
            #core::Inject(db): #core::Inject<#pg::PgClient>,
            #core::Path(id): #core::Path<#id_type>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            let result: Result<bool, #pg::PgError> =
                <#pg::PgClient as #pg::PgRepository<#name>>::delete(&*db, &id).await;

            match result {
                Ok(true) => {
                    axum::http::StatusCode::NO_CONTENT.into_response()
                }
                Ok(false) => {
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
            method: "DELETE",
            handler: || axum::routing::delete(#handler_name),
        });
    };

    (handler, registration)
}
