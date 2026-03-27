use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use crate::crud::parse::CrudEntityInfo;

/// Generate the list handler (GET /{path}).
pub(super) fn generate_list_handler(entity: &CrudEntityInfo, path: &str, core: &TokenStream2, pg: &TokenStream2) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_list", snake_name);
    let query_name = format_ident!("{}Query", name);
    let response_name = format_ident!("{}Response", name);

    let default_page_limit = super::DEFAULT_PAGE_LIMIT;

    let count_err = super::internal_error_response(core, &snake_name, "count");
    let fetch_err = super::internal_error_response(core, &snake_name, "fetch");

    let handler = quote! {
        async fn #handler_name(
            #core::Inject(db): #core::Inject<#pg::PgClient>,
            #core::Query(query): #core::Query<#query_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            let limit = query.limit.unwrap_or(#default_page_limit);
            let offset = query.offset.unwrap_or(0);

            let total: i64 = match <#pg::PgClient as #pg::PgRepository<#name>>::count(&*db).await {
                Ok(count) => count,
                Err(e) => return #count_err,
            };

            let rows = match <#pg::PgClient as #pg::PgRepository<#name>>::find_page(&*db, limit, offset).await {
                Ok(rows) => rows,
                Err(e) => return #fetch_err,
            };

            let data: Vec<#response_name> = rows
                .into_iter()
                .map(#response_name::from)
                .collect();

            let response = #core::crud::PagedResponse::new(data, total, Some(limit), Some(offset));
            #core::Json(response).into_response()
        }
    };

    let registration = quote! {
        #core::inventory::submit!(#core::RouteRegistration {
            path: #path,
            method: "GET",
            handler: || axum::routing::get(#handler_name),
        });
    };

    (handler, registration)
}
