mod parse;
mod repository;
mod sql;

use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, parse_macro_input};

use parse::parse_entity;
use repository::generate_repository_impl;
use sql::*;

pub fn generate_pg_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    generate_pg_entity_internal(input).unwrap_or_else(|e| e.to_compile_error().into())
}

fn generate_pg_entity_internal(input: DeriveInput) -> Result<TokenStream, Error> {
    let entity = parse_entity(&input)?;

    let core = crate::paths::core_crate();
    let pg = crate::paths::pg_crate();

    let name = &entity.name;
    let id_type = generate_id_type(&entity);
    let id_fn = generate_id_fn(&entity);
    let from_row = generate_from_row(&entity, &pg);

    let columns: Vec<_> = generate_columns(&entity);
    let pk_columns: Vec<_> = generate_pk_columns(&entity);
    let table = &entity.table;

    let schema_const = entity.schema.as_ref().map(|s| {
        quote! { const SCHEMA: &'static str = #s; }
    });

    let entity_impl = quote! {
        impl #pg::PgEntity for #name {
            type Id = #id_type;

            const TABLE: &'static str = #table;
            #schema_const
            const COLUMNS: &'static [&'static str] = &[#(#columns),*];
            const PK_COLUMNS: &'static [&'static str] = &[#(#pk_columns),*];

            fn id(&self) -> Self::Id {
                #id_fn
            }

            fn from_row(row: #pg::PgRow) -> Self {
                #from_row
            }
        }
    };

    let repository_impl = generate_repository_impl(&entity, &core, &pg);

    let expanded = quote! {
        #entity_impl
        #repository_impl
    };

    Ok(expanded.into())
}
