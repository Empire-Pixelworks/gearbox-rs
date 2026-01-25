//! Route handler generation for CRUD operations.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use super::parse::CrudEntityInfo;

/// Generate all route handlers for the entity.
pub fn generate_routes(entity: &CrudEntityInfo) -> TokenStream2 {
    let path = entity.effective_path();

    // Generate handlers based on config
    let mut handlers = Vec::new();
    let mut registrations = Vec::new();

    // GET list - always generated
    let (list_handler, list_reg) = generate_list_handler(entity, &path);
    handlers.push(list_handler);
    registrations.push(list_reg);

    // GET by id - always generated
    let (get_handler, get_reg) = generate_get_handler(entity, &path);
    handlers.push(get_handler);
    registrations.push(get_reg);

    // POST create - unless read_only or skip_create
    if !entity.config.read_only && !entity.config.skip_create {
        let (create_handler, create_reg) = generate_create_handler(entity, &path);
        handlers.push(create_handler);
        registrations.push(create_reg);
    }

    // PUT update (full) - unless read_only
    if !entity.config.read_only {
        let (put_handler, put_reg) = generate_put_handler(entity, &path);
        handlers.push(put_handler);
        registrations.push(put_reg);
    }

    // PATCH update (partial) - unless read_only
    if !entity.config.read_only {
        let (patch_handler, patch_reg) = generate_patch_handler(entity, &path);
        handlers.push(patch_handler);
        registrations.push(patch_reg);
    }

    // DELETE - unless read_only or skip_delete
    if !entity.config.read_only && !entity.config.skip_delete {
        let (delete_handler, delete_reg) = generate_delete_handler(entity, &path);
        handlers.push(delete_handler);
        registrations.push(delete_reg);
    }

    quote! {
        #(#handlers)*
        #(#registrations)*
    }
}

/// Generate error response JSON.
fn error_json() -> TokenStream2 {
    // Use a simple struct for error responses instead of serde_json::json!
    quote! {
        #[derive(serde::Serialize)]
        struct __CrudErrorResponse {
            error: String,
        }
    }
}

/// Generate the list handler (GET /{path}).
fn generate_list_handler(entity: &CrudEntityInfo, path: &str) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_list", snake_name);
    let query_name = format_ident!("{}Query", name);
    let response_name = format_ident!("{}Response", name);
    let table = &entity.table;

    let columns: Vec<String> = entity
        .db_fields()
        .iter()
        .map(|f| f.ident.to_string())
        .collect();
    let columns_str = columns.join(", ");

    let pk_columns: Vec<String> = entity
        .pk_fields()
        .iter()
        .map(|f| f.ident.to_string())
        .collect();
    let default_order = pk_columns.join(", ");

    let error_struct = error_json();

    let handler = quote! {
        async fn #handler_name(
            gearbox_rs_core::Inject(db): gearbox_rs_core::Inject<gearbox_rs_postgres::PgClient>,
            gearbox_rs_core::Query(query): gearbox_rs_core::Query<#query_name>,
        ) -> axum::response::Response {
            use gearbox_rs_core::crud::BuildWhereClause;
            use axum::response::IntoResponse;

            #error_struct

            let (conditions, _params) = query.build_conditions();
            let (limit, offset) = query.pagination();

            // Build the query
            let where_clause = if conditions.is_empty() {
                String::new()
            } else {
                format!("WHERE {}", conditions.join(" AND "))
            };

            let order_by = query.sort_spec()
                .map(|s| format!("ORDER BY {}", s.to_sql()))
                .unwrap_or_else(|| format!("ORDER BY {}", #default_order));

            let limit_clause = limit.map(|l| format!("LIMIT {}", l)).unwrap_or_default();
            let offset_clause = offset.map(|o| format!("OFFSET {}", o)).unwrap_or_default();

            // Count query
            let count_sql = format!("SELECT COUNT(*) FROM {} {}", #table, where_clause);
            let total: i64 = match gearbox_rs_postgres::query_scalar(&count_sql)
                .fetch_one(db.pool.as_ref())
                .await
            {
                Ok(count) => count,
                Err(e) => return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    gearbox_rs_core::Json(__CrudErrorResponse { error: format!("Failed to count: {}", e) })
                ).into_response(),
            };

            // Data query
            let data_sql = format!(
                "SELECT {} FROM {} {} {} {} {}",
                #columns_str, #table, where_clause, order_by, limit_clause, offset_clause
            );

            let rows = match gearbox_rs_postgres::query(&data_sql)
                .fetch_all(db.pool.as_ref())
                .await
            {
                Ok(rows) => rows,
                Err(e) => return (
                    axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                    gearbox_rs_core::Json(__CrudErrorResponse { error: format!("Failed to fetch: {}", e) })
                ).into_response(),
            };

            let data: Vec<#response_name> = rows
                .into_iter()
                .map(|row| {
                    let entity = <#name as gearbox_rs_postgres::PgEntity>::from_row(row);
                    #response_name::from(entity)
                })
                .collect();

            let response = gearbox_rs_core::crud::PagedResponse::new(data, total, limit, offset);
            gearbox_rs_core::Json(response).into_response()
        }
    };

    let registration = quote! {
        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #path,
            method: "GET",
            handler: || axum::routing::get(#handler_name),
        });
    };

    (handler, registration)
}

/// Generate the get handler (GET /{path}/{id}).
fn generate_get_handler(entity: &CrudEntityInfo, path: &str) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_get", snake_name);
    let response_name = format_ident!("{}Response", name);

    let pk_fields = entity.pk_fields();
    let id_path = format!("{}/{{id}}", path);

    // For now, support single-column primary keys
    let id_type = if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        // For composite keys, use String and parse
        quote! { String }
    };

    let error_struct = error_json();

    let handler = quote! {
        async fn #handler_name(
            gearbox_rs_core::Inject(db): gearbox_rs_core::Inject<gearbox_rs_postgres::PgClient>,
            gearbox_rs_core::Path(id): gearbox_rs_core::Path<#id_type>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #error_struct

            let result: Result<Option<#name>, gearbox_rs_postgres::PgError> =
                <gearbox_rs_postgres::PgClient as gearbox_rs_postgres::PgRepository<#name>>::find_by_id(&*db, &id).await;

            match result {
                Ok(Some(entity)) => {
                    let response = #response_name::from(entity);
                    (axum::http::StatusCode::OK, gearbox_rs_core::Json(response)).into_response()
                }
                Ok(None) => {
                    (axum::http::StatusCode::NOT_FOUND, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: "Not found".to_string()
                    })).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: format!("{}", e)
                    })).into_response()
                }
            }
        }
    };

    let registration = quote! {
        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #id_path,
            method: "GET",
            handler: || axum::routing::get(#handler_name),
        });
    };

    (handler, registration)
}

/// Generate the create handler (POST /{path}).
fn generate_create_handler(entity: &CrudEntityInfo, path: &str) -> (TokenStream2, TokenStream2) {
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
                let ty_str = quote!(#ty).to_string();
                if ty_str.contains("Uuid") {
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

    let error_struct = error_json();

    let handler = quote! {
        async fn #handler_name(
            gearbox_rs_core::Inject(db): gearbox_rs_core::Inject<gearbox_rs_postgres::PgClient>,
            gearbox_rs_core::Json(body): gearbox_rs_core::Json<#create_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #error_struct

            #entity_creation

            let result: Result<#name, gearbox_rs_postgres::PgError> =
                <gearbox_rs_postgres::PgClient as gearbox_rs_postgres::PgRepository<#name>>::create(&*db, entity).await;

            match result {
                Ok(created) => {
                    let response = #response_name::from(created);
                    (axum::http::StatusCode::CREATED, gearbox_rs_core::Json(response)).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: format!("{}", e)
                    })).into_response()
                }
            }
        }
    };

    let registration = quote! {
        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #path,
            method: "POST",
            handler: || axum::routing::post(#handler_name),
        });
    };

    (handler, registration)
}

/// Generate the full update handler (PUT /{path}/{id}).
fn generate_put_handler(entity: &CrudEntityInfo, path: &str) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_update", snake_name);
    let create_name = format_ident!("{}Create", name);
    let response_name = format_ident!("{}Response", name);
    let id_path = format!("{}/{{id}}", path);

    let pk_fields = entity.pk_fields();
    let id_type = if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        quote! { String }
    };

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
                    let ty_str = quote!(#ty).to_string();
                    if ty_str.contains("Uuid") {
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

    let error_struct = error_json();

    let handler = quote! {
        async fn #handler_name(
            gearbox_rs_core::Inject(db): gearbox_rs_core::Inject<gearbox_rs_postgres::PgClient>,
            gearbox_rs_core::Path(id): gearbox_rs_core::Path<#id_type>,
            gearbox_rs_core::Json(body): gearbox_rs_core::Json<#create_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #error_struct

            #entity_creation

            let result: Result<#name, gearbox_rs_postgres::PgError> =
                <gearbox_rs_postgres::PgClient as gearbox_rs_postgres::PgRepository<#name>>::update(&*db, entity).await;

            match result {
                Ok(updated) => {
                    let response = #response_name::from(updated);
                    (axum::http::StatusCode::OK, gearbox_rs_core::Json(response)).into_response()
                }
                Err(gearbox_rs_postgres::PgError::NotFound) => {
                    (axum::http::StatusCode::NOT_FOUND, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: "Not found".to_string()
                    })).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: format!("{}", e)
                    })).into_response()
                }
            }
        }
    };

    let registration = quote! {
        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #id_path,
            method: "PUT",
            handler: || axum::routing::put(#handler_name),
        });
    };

    (handler, registration)
}

/// Generate the partial update handler (PATCH /{path}/{id}).
fn generate_patch_handler(entity: &CrudEntityInfo, path: &str) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_patch", snake_name);
    let update_name = format_ident!("{}Update", name);
    let response_name = format_ident!("{}Response", name);
    let id_path = format!("{}/{{id}}", path);

    let pk_fields = entity.pk_fields();
    let id_type = if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        quote! { String }
    };

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

    let error_struct = error_json();

    let handler = quote! {
        async fn #handler_name(
            gearbox_rs_core::Inject(db): gearbox_rs_core::Inject<gearbox_rs_postgres::PgClient>,
            gearbox_rs_core::Path(id): gearbox_rs_core::Path<#id_type>,
            gearbox_rs_core::Json(body): gearbox_rs_core::Json<#update_name>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #error_struct

            // First, fetch the existing entity
            let find_result: Result<Option<#name>, gearbox_rs_postgres::PgError> =
                <gearbox_rs_postgres::PgClient as gearbox_rs_postgres::PgRepository<#name>>::find_by_id(&*db, &id).await;

            let mut existing = match find_result {
                Ok(Some(entity)) => entity,
                Ok(None) => {
                    return (axum::http::StatusCode::NOT_FOUND, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: "Not found".to_string()
                    })).into_response();
                }
                Err(e) => {
                    return (axum::http::StatusCode::INTERNAL_SERVER_ERROR, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: format!("{}", e)
                    })).into_response();
                }
            };

            // Apply partial updates
            #(#field_updates)*

            // Save the updated entity
            let update_result: Result<#name, gearbox_rs_postgres::PgError> =
                <gearbox_rs_postgres::PgClient as gearbox_rs_postgres::PgRepository<#name>>::update(&*db, existing).await;

            match update_result {
                Ok(updated) => {
                    let response = #response_name::from(updated);
                    (axum::http::StatusCode::OK, gearbox_rs_core::Json(response)).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: format!("{}", e)
                    })).into_response()
                }
            }
        }
    };

    let registration = quote! {
        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #id_path,
            method: "PATCH",
            handler: || axum::routing::patch(#handler_name),
        });
    };

    (handler, registration)
}

/// Generate the delete handler (DELETE /{path}/{id}).
fn generate_delete_handler(entity: &CrudEntityInfo, path: &str) -> (TokenStream2, TokenStream2) {
    let name = &entity.name;
    let snake_name = entity.snake_case_name();
    let handler_name = format_ident!("__crud_{}_delete", snake_name);
    let id_path = format!("{}/{{id}}", path);

    let pk_fields = entity.pk_fields();
    let id_type = if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        quote! { String }
    };

    let error_struct = error_json();

    let handler = quote! {
        async fn #handler_name(
            gearbox_rs_core::Inject(db): gearbox_rs_core::Inject<gearbox_rs_postgres::PgClient>,
            gearbox_rs_core::Path(id): gearbox_rs_core::Path<#id_type>,
        ) -> axum::response::Response {
            use axum::response::IntoResponse;

            #error_struct

            let result: Result<bool, gearbox_rs_postgres::PgError> =
                <gearbox_rs_postgres::PgClient as gearbox_rs_postgres::PgRepository<#name>>::delete(&*db, &id).await;

            match result {
                Ok(true) => {
                    axum::http::StatusCode::NO_CONTENT.into_response()
                }
                Ok(false) => {
                    (axum::http::StatusCode::NOT_FOUND, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: "Not found".to_string()
                    })).into_response()
                }
                Err(e) => {
                    (axum::http::StatusCode::INTERNAL_SERVER_ERROR, gearbox_rs_core::Json(__CrudErrorResponse {
                        error: format!("{}", e)
                    })).into_response()
                }
            }
        }
    };

    let registration = quote! {
        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #id_path,
            method: "DELETE",
            handler: || axum::routing::delete(#handler_name),
        });
    };

    (handler, registration)
}
