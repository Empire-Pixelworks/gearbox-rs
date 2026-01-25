//! DTO generation for CRUD operations.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::Type;

use super::parse::{CrudEntityInfo, CrudFieldInfo, SearchOperator};

/// Generate all DTOs for the entity.
pub fn generate_dtos(entity: &CrudEntityInfo) -> TokenStream2 {
    let create_dto = generate_create_dto(entity);
    let update_dto = generate_update_dto(entity);
    let query_dto = generate_query_dto(entity);
    let response_dto = generate_response_dto(entity);
    let conversions = generate_conversions(entity);

    quote! {
        #create_dto
        #update_dto
        #query_dto
        #response_dto
        #conversions
    }
}

/// Generate the Create DTO.
fn generate_create_dto(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let create_name = format_ident!("{}Create", name);
    let fields = entity.create_fields();

    if fields.is_empty() {
        return quote! {};
    }

    let field_defs: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { pub #ident: #ty }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, serde::Deserialize)]
        #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
        pub struct #create_name {
            #(#field_defs),*
        }
    }
}

/// Generate the Update DTO (all fields optional for partial updates).
fn generate_update_dto(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let update_name = format_ident!("{}Update", name);
    let fields = entity.update_fields();

    if fields.is_empty() {
        return quote! {};
    }

    let field_defs: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            // Wrap in Option for partial updates, unless already Option
            let ty_str = quote!(#ty).to_string();
            if ty_str.starts_with("Option") {
                quote! { pub #ident: #ty }
            } else {
                quote! { pub #ident: Option<#ty> }
            }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, Default, serde::Deserialize)]
        #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
        pub struct #update_name {
            #(#field_defs),*
        }
    }
}

/// Generate the Query DTO with searchable fields and pagination.
fn generate_query_dto(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let query_name = format_ident!("{}Query", name);
    let searchable_fields = entity.searchable_fields();

    let mut field_defs: Vec<TokenStream2> = Vec::new();

    // Add pagination fields
    field_defs.push(quote! {
        #[serde(default)]
        pub limit: Option<i64>
    });
    field_defs.push(quote! {
        #[serde(default)]
        pub offset: Option<i64>
    });
    field_defs.push(quote! {
        #[serde(default)]
        pub sort: Option<String>
    });

    // Add searchable field parameters with operator suffixes
    for field in &searchable_fields {
        let base_ident = &field.ident;
        let ty = &field.ty;

        if let Some(ref searchable) = field.searchable {
            for op in &searchable.operators {
                let suffix = op.suffix();
                let field_name = if suffix.is_empty() {
                    base_ident.clone()
                } else {
                    format_ident!("{}{}", base_ident, suffix)
                };

                // Determine the query param type
                let param_ty = get_query_param_type(ty, op);

                field_defs.push(quote! {
                    #[serde(default)]
                    pub #field_name: Option<#param_ty>
                });
            }
        }
    }

    quote! {
        #[derive(Debug, Clone, Default, serde::Deserialize)]
        #[cfg_attr(feature = "openapi", derive(utoipa::IntoParams))]
        pub struct #query_name {
            #(#field_defs),*
        }
    }
}

/// Get the appropriate query parameter type for a field and operator.
fn get_query_param_type(ty: &Type, op: &SearchOperator) -> TokenStream2 {
    let ty_str = quote!(#ty).to_string();

    match op {
        // String operations always take String
        SearchOperator::Like | SearchOperator::StartsWith => {
            quote! { String }
        }
        // For equality and comparison, use the original type
        _ => {
            // If the original type is Option<T>, extract T
            if ty_str.starts_with("Option <") {
                // Extract inner type
                let inner_start = ty_str.find('<').unwrap() + 1;
                let inner_end = ty_str.rfind('>').unwrap();
                let inner = &ty_str[inner_start..inner_end].trim();
                let inner_ty: Type = syn::parse_str(inner).unwrap_or_else(|_| ty.clone());
                quote! { #inner_ty }
            } else {
                quote! { #ty }
            }
        }
    }
}

/// Generate the Response DTO.
fn generate_response_dto(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let response_name = format_ident!("{}Response", name);
    let fields = entity.response_fields();

    let field_defs: Vec<TokenStream2> = fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { pub #ident: #ty }
        })
        .collect();

    quote! {
        #[derive(Debug, Clone, serde::Serialize)]
        #[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
        pub struct #response_name {
            #(#field_defs),*
        }
    }
}

/// Generate conversion implementations.
fn generate_conversions(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let response_name = format_ident!("{}Response", name);

    let response_fields = entity.response_fields();

    // Entity -> Response conversion
    let response_field_mappings: Vec<TokenStream2> = response_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            quote! { #ident: entity.#ident }
        })
        .collect();

    // Create -> Entity conversion (for fields that come from Create DTO)
    // This requires handling auto_generated and readonly fields
    let entity_from_create = generate_entity_from_create(entity);

    quote! {
        impl From<#name> for #response_name {
            fn from(entity: #name) -> Self {
                #response_name {
                    #(#response_field_mappings),*
                }
            }
        }

        impl From<&#name> for #response_name {
            fn from(entity: &#name) -> Self {
                #response_name {
                    #(#response_field_mappings.clone()),*
                }
            }
        }

        #entity_from_create
    }
}

/// Generate entity construction from Create DTO.
fn generate_entity_from_create(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let create_name = format_ident!("{}Create", name);

    // For auto-generated fields, we need to provide a default or let the builder handle it
    // This is a partial conversion - the caller needs to provide the auto-generated fields
    if entity.create_fields().is_empty() {
        return quote! {};
    }

    // Generate a helper method on Create DTO to build an entity
    // Auto-generated fields will need to be provided separately
    let auto_gen_fields: Vec<&CrudFieldInfo> = entity
        .fields
        .iter()
        .filter(|f| f.auto_generated && !f.skip)
        .collect();

    // Build the entity constructor parameters (auto-generated fields)
    let constructor_params: Vec<TokenStream2> = auto_gen_fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let ty = &f.ty;
            quote! { #ident: #ty }
        })
        .collect();

    // Build field assignments
    let field_assignments: Vec<TokenStream2> = entity
        .db_fields()
        .iter()
        .map(|f| {
            let ident = &f.ident;
            if f.auto_generated {
                // Use the provided parameter
                quote! { #ident }
            } else if f.readonly {
                // Use Default for readonly fields
                quote! { #ident: Default::default() }
            } else if f.skip {
                // Use Default for skipped fields
                quote! { #ident: Default::default() }
            } else {
                // Use from create
                quote! { #ident: self.#ident }
            }
        })
        .collect();

    if auto_gen_fields.is_empty() {
        quote! {
            impl #create_name {
                pub fn into_entity(self) -> #name {
                    #name {
                        #(#field_assignments),*
                    }
                }
            }
        }
    } else {
        quote! {
            impl #create_name {
                pub fn into_entity(self, #(#constructor_params),*) -> #name {
                    #name {
                        #(#field_assignments),*
                    }
                }
            }
        }
    }
}

/// Generate the BuildWhereClause implementation for the Query DTO.
pub fn generate_where_clause_impl(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let query_name = format_ident!("{}Query", name);
    let searchable_fields = entity.searchable_fields();

    let mut condition_checks: Vec<TokenStream2> = Vec::new();

    for field in &searchable_fields {
        let base_ident = &field.ident;
        let col_name = base_ident.to_string();

        if let Some(ref searchable) = field.searchable {
            for op in &searchable.operators {
                let suffix = op.suffix();
                let field_name = if suffix.is_empty() {
                    base_ident.clone()
                } else {
                    format_ident!("{}{}", base_ident, suffix)
                };

                let sql_op = op.sql_operator();

                let condition = match op {
                    SearchOperator::Like => {
                        quote! {
                            if let Some(ref val) = self.#field_name {
                                let param_idx = params.len() + 1;
                                conditions.push(format!("{} ILIKE ${}", #col_name, param_idx));
                                params.push(format!("%{}%", val));
                            }
                        }
                    }
                    SearchOperator::StartsWith => {
                        quote! {
                            if let Some(ref val) = self.#field_name {
                                let param_idx = params.len() + 1;
                                conditions.push(format!("{} ILIKE ${}", #col_name, param_idx));
                                params.push(format!("{}%", val));
                            }
                        }
                    }
                    _ => {
                        quote! {
                            if let Some(ref val) = self.#field_name {
                                let param_idx = params.len() + 1;
                                conditions.push(format!("{} {} ${}", #col_name, #sql_op, param_idx));
                                params.push(val.to_string());
                            }
                        }
                    }
                };

                condition_checks.push(condition);
            }
        }
    }

    quote! {
        impl gearbox_rs_core::crud::BuildWhereClause for #query_name {
            fn build_conditions(&self) -> (Vec<String>, Vec<String>) {
                let mut conditions: Vec<String> = Vec::new();
                let mut params: Vec<String> = Vec::new();

                #(#condition_checks)*

                (conditions, params)
            }

            fn pagination(&self) -> (Option<i64>, Option<i64>) {
                (self.limit, self.offset)
            }

            fn sort_spec(&self) -> Option<gearbox_rs_core::crud::SortSpec> {
                self.sort.as_ref().map(|s| gearbox_rs_core::crud::SortSpec::parse(s))
            }
        }
    }
}
