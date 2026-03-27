use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::crud::parse::CrudEntityInfo;

/// Generate the create handler (POST /{path}).
pub(super) fn generate_create_handler(entity: &CrudEntityInfo, path: &str, core: &TokenStream2, pg: &TokenStream2) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_create", snake_name);
    let create_name = format_ident!("{}Create", name);
    let response_name = format_ident!("{}Response", name);

    // Check if there are auto-generated fields
    let auto_gen_fields: Vec<_> = entity
        .fields
        .iter()
        .filter(|f| f.auto_generated && !f.skip)
        .collect();

    let entity_creation = if auto_gen_fields.is_empty() {
        quote! {
            let entity = body.into_entity();
        }
    } else {
        // Generate default values for auto-generated fields
        let auto_gen_values: Vec<TokenStream2> = auto_gen_fields
            .iter()
            .map(|f| {
                let ty = &f.ty;
                if super::type_is_uuid(ty) {
                    quote! { uuid::Uuid::new_v4() }
                } else {
                    quote! { Default::default() }
                }
            })
            .collect();
        quote! {
            let entity = body.into_entity(#(#auto_gen_values),*);
        }
    };

    let err_response = super::internal_error_response(core, &snake_name, "create");

    let handler = quote! {
        async fn #handler_name(
            #core::Inject(db): #core::Inject<#pg::PgClient>,
            #core::Json(body): #core::Json<#create_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #entity_creation

            let result: Result<#name, #pg::PgError> =
                <#pg::PgClient as #pg::PgRepository<#name>>::create(&*db, entity).await;

            match result {
                Ok(created) => {
                    let response = #response_name::from(created);
                    (axum::http::StatusCode::CREATED, #core::Json(response)).into_response()
                }
                Err(e) => {
                    #err_response
                }
            }
        }
    };

    let registration = quote! {
        #core::inventory::submit!(#core::RouteRegistration {
            path: #path,
            method: "POST",
            handler: || axum::routing::post(#handler_name),
        });
    };

    (handler, registration)
}
