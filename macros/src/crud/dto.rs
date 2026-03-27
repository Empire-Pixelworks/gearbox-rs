//! DTO generation for CRUD operations.

use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

use super::parse::{CrudEntityInfo, CrudFieldInfo};

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
        pub struct #update_name {
            #(#field_defs),*
        }
    }
}

/// Generate the Query DTO with pagination only.
fn generate_query_dto(entity: &CrudEntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let query_name = format_ident!("{}Query", name);

    quote! {
        #[derive(Debug, Clone, Default, serde::Deserialize)]
        pub struct #query_name {
            #[serde(default)]
            pub limit: Option<i64>,
            #[serde(default)]
            pub offset: Option<i64>,
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
