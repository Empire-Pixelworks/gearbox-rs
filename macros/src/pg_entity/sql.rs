use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::parse::{EntityInfo, FieldInfo};

pub(super) fn generate_id_type(entity: &EntityInfo) -> TokenStream2 {
    let pk_fields = entity.pk_fields();
    if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        let types: Vec<_> = pk_fields.iter().map(|f| &f.ty).collect();
        quote! { (#(#types),*) }
    }
}

pub(super) fn generate_id_fn(entity: &EntityInfo) -> TokenStream2 {
    let pk_fields = entity.pk_fields();
    if pk_fields.len() == 1 {
        let ident = &pk_fields[0].ident;
        quote! { self.#ident.clone() }
    } else {
        let idents: Vec<_> = pk_fields.iter().map(|f| &f.ident).collect();
        quote! { (#(self.#idents.clone()),*) }
    }
}

pub(super) fn generate_from_row(entity: &EntityInfo, pg: &TokenStream2) -> TokenStream2 {
    let name = &entity.name;
    let field_inits: Vec<_> = entity
        .fields
        .iter()
        .map(|f| {
            let ident = &f.ident;
            let col_name = ident.to_string();
            if f.skip {
                quote! { #ident: Default::default() }
            } else {
                quote! { #ident: #pg::Row::get(&row, #col_name) }
            }
        })
        .collect();

    quote! {
        #name {
            #(#field_inits),*
        }
    }
}

pub(super) fn generate_columns(entity: &EntityInfo) -> Vec<String> {
    entity
        .db_fields()
        .iter()
        .map(|f| f.ident.to_string())
        .collect()
}

pub(super) fn generate_pk_columns(entity: &EntityInfo) -> Vec<String> {
    entity
        .pk_fields()
        .iter()
        .map(|f| f.ident.to_string())
        .collect()
}

pub(super) fn generate_bind_value(field: &FieldInfo) -> TokenStream2 {
    let ident = &field.ident;
    if let Some(ref pg_ty) = field.pg_type {
        quote! { &entity.#ident as &#pg_ty }
    } else {
        quote! { &entity.#ident }
    }
}

pub(super) fn generate_insert_sql(entity: &EntityInfo) -> String {
    let columns = generate_columns(entity);
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

    format!(
        "INSERT INTO {} ({}) VALUES ({})",
        entity.table,
        columns.join(", "),
        placeholders.join(", ")
    )
}

pub(super) fn generate_upsert_sql(entity: &EntityInfo) -> String {
    let columns = generate_columns(entity);
    let pk_columns = generate_pk_columns(entity);
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

    let update_fields = entity.upsert_update_fields();
    let updates: Vec<String> = update_fields
        .iter()
        .map(|f| {
            let col = f.ident.to_string();
            format!("{} = EXCLUDED.{}", col, col)
        })
        .collect();

    if updates.is_empty() {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO NOTHING",
            entity.table,
            columns.join(", "),
            placeholders.join(", "),
            pk_columns.join(", ")
        )
    } else {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) ON CONFLICT ({}) DO UPDATE SET {}",
            entity.table,
            columns.join(", "),
            placeholders.join(", "),
            pk_columns.join(", "),
            updates.join(", ")
        )
    }
}

pub(super) fn generate_update_sql(entity: &EntityInfo) -> String {
    let non_pk = entity.non_pk_fields();
    let pk_columns = generate_pk_columns(entity);

    let mut param_idx = 1usize;
    let sets: Vec<String> = non_pk
        .iter()
        .map(|f| {
            let col = f.ident.to_string();
            let s = format!("{} = ${}", col, param_idx);
            param_idx += 1;
            s
        })
        .collect();

    let where_clause: Vec<String> = pk_columns
        .iter()
        .map(|col| {
            let s = format!("{} = ${}", col, param_idx);
            param_idx += 1;
            s
        })
        .collect();

    format!(
        "UPDATE {} SET {} WHERE {}",
        entity.table,
        sets.join(", "),
        where_clause.join(" AND ")
    )
}

pub(super) fn generate_select_by_id_sql(entity: &EntityInfo) -> String {
    let columns = generate_columns(entity);
    let pk_columns = generate_pk_columns(entity);

    let where_clause: Vec<String> = pk_columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{} = ${}", col, i + 1))
        .collect();

    format!(
        "SELECT {} FROM {} WHERE {}",
        columns.join(", "),
        entity.table,
        where_clause.join(" AND ")
    )
}

pub(super) fn generate_delete_sql(entity: &EntityInfo) -> String {
    let pk_columns = generate_pk_columns(entity);

    let where_clause: Vec<String> = pk_columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{} = ${}", col, i + 1))
        .collect();

    format!(
        "DELETE FROM {} WHERE {}",
        entity.table,
        where_clause.join(" AND ")
    )
}

pub(super) fn generate_count_sql(entity: &EntityInfo) -> String {
    format!("SELECT COUNT(*) FROM {}", entity.table)
}

pub(super) fn generate_exists_sql(entity: &EntityInfo) -> String {
    let pk_columns = generate_pk_columns(entity);

    let where_clause: Vec<String> = pk_columns
        .iter()
        .enumerate()
        .map(|(i, col)| format!("{} = ${}", col, i + 1))
        .collect();

    format!(
        "SELECT EXISTS(SELECT 1 FROM {} WHERE {})",
        entity.table,
        where_clause.join(" AND ")
    )
}

pub(super) fn generate_find_page_sql(entity: &EntityInfo) -> String {
    let columns = generate_columns(entity);
    let pk_columns = generate_pk_columns(entity);

    format!(
        "SELECT {} FROM {} ORDER BY {} LIMIT $1 OFFSET $2",
        columns.join(", "),
        entity.table,
        pk_columns.join(", ")
    )
}

pub(super) fn generate_pk_bind(entity: &EntityInfo) -> TokenStream2 {
    let pk_fields = entity.pk_fields();
    if pk_fields.len() == 1 {
        quote! { .bind(id) }
    } else {
        let binds: Vec<_> = (0..pk_fields.len())
            .map(|i| {
                let idx = syn::Index::from(i);
                quote! { .bind(&id.#idx) }
            })
            .collect();
        quote! { #(#binds)* }
    }
}
