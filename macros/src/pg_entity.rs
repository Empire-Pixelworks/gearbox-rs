use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Fields, Ident, Type, parse_macro_input};

struct FieldInfo {
    ident: Ident,
    ty: Type,
    is_primary_key: bool,
    skip: bool,
    skip_upsert: bool,
    pg_type: Option<Type>,
}

struct EntityInfo {
    name: Ident,
    table: String,
    fields: Vec<FieldInfo>,
}

impl EntityInfo {
    fn pk_fields(&self) -> Vec<&FieldInfo> {
        self.fields.iter().filter(|f| f.is_primary_key).collect()
    }

    fn db_fields(&self) -> Vec<&FieldInfo> {
        self.fields.iter().filter(|f| !f.skip).collect()
    }

    fn non_pk_fields(&self) -> Vec<&FieldInfo> {
        self.fields
            .iter()
            .filter(|f| !f.skip && !f.is_primary_key)
            .collect()
    }

    fn upsert_update_fields(&self) -> Vec<&FieldInfo> {
        self.fields
            .iter()
            .filter(|f| !f.skip && !f.is_primary_key && !f.skip_upsert)
            .collect()
    }
}

fn parse_entity(input: &DeriveInput) -> EntityInfo {
    let name = input.ident.clone();

    // Parse #[table("name")] attribute
    let table = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("table") {
                attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
            } else {
                None
            }
        })
        .expect("#[table(\"name\")] attribute is required");

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| {
                    let ident = f.ident.clone().unwrap();
                    let ty = f.ty.clone();

                    let mut is_primary_key = false;
                    let mut skip = false;
                    let mut skip_upsert = false;
                    let mut pg_type = None;

                    for attr in &f.attrs {
                        if attr.path().is_ident("primary_key") {
                            is_primary_key = true;
                        } else if attr.path().is_ident("skip") {
                            skip = true;
                        } else if attr.path().is_ident("skip_upsert") {
                            skip_upsert = true;
                        } else if attr.path().is_ident("pg_type") {
                            pg_type = attr.parse_args::<Type>().ok();
                        }
                    }

                    FieldInfo {
                        ident,
                        ty,
                        is_primary_key,
                        skip,
                        skip_upsert,
                        pg_type,
                    }
                })
                .collect(),
            _ => panic!("PgEntity can only be derived for structs with named fields"),
        },
        _ => panic!("PgEntity can only be derived for structs"),
    };

    EntityInfo {
        name,
        table,
        fields,
    }
}

fn generate_id_type(entity: &EntityInfo) -> TokenStream2 {
    let pk_fields = entity.pk_fields();
    if pk_fields.len() == 1 {
        let ty = &pk_fields[0].ty;
        quote! { #ty }
    } else {
        let types: Vec<_> = pk_fields.iter().map(|f| &f.ty).collect();
        quote! { (#(#types),*) }
    }
}

fn generate_id_fn(entity: &EntityInfo) -> TokenStream2 {
    let pk_fields = entity.pk_fields();
    if pk_fields.len() == 1 {
        let ident = &pk_fields[0].ident;
        quote! { self.#ident.clone() }
    } else {
        let idents: Vec<_> = pk_fields.iter().map(|f| &f.ident).collect();
        quote! { (#(self.#idents.clone()),*) }
    }
}

fn generate_from_row(entity: &EntityInfo) -> TokenStream2 {
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
                quote! { #ident: gearbox_rs_postgres::Row::get(&row, #col_name) }
            }
        })
        .collect();

    quote! {
        #name {
            #(#field_inits),*
        }
    }
}

fn generate_columns(entity: &EntityInfo) -> Vec<String> {
    entity
        .db_fields()
        .iter()
        .map(|f| f.ident.to_string())
        .collect()
}

fn generate_pk_columns(entity: &EntityInfo) -> Vec<String> {
    entity
        .pk_fields()
        .iter()
        .map(|f| f.ident.to_string())
        .collect()
}

fn generate_bind_value(field: &FieldInfo) -> TokenStream2 {
    let ident = &field.ident;
    if let Some(ref pg_ty) = field.pg_type {
        quote! { &entity.#ident as &#pg_ty }
    } else {
        quote! { &entity.#ident }
    }
}

fn generate_insert_sql(entity: &EntityInfo) -> String {
    let columns = generate_columns(entity);
    let placeholders: Vec<String> = (1..=columns.len()).map(|i| format!("${}", i)).collect();

    format!(
        "INSERT INTO {} ({}) VALUES ({})",
        entity.table,
        columns.join(", "),
        placeholders.join(", ")
    )
}

fn generate_upsert_sql(entity: &EntityInfo) -> String {
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

fn generate_update_sql(entity: &EntityInfo) -> String {
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

fn generate_select_by_id_sql(entity: &EntityInfo) -> String {
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

fn generate_delete_sql(entity: &EntityInfo) -> String {
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

fn generate_count_sql(entity: &EntityInfo) -> String {
    format!("SELECT COUNT(*) FROM {}", entity.table)
}

fn generate_exists_sql(entity: &EntityInfo) -> String {
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

fn generate_find_page_sql(entity: &EntityInfo) -> String {
    let columns = generate_columns(entity);
    let pk_columns = generate_pk_columns(entity);

    format!(
        "SELECT {} FROM {} ORDER BY {} LIMIT $1 OFFSET $2",
        columns.join(", "),
        entity.table,
        pk_columns.join(", ")
    )
}

fn generate_pk_bind(entity: &EntityInfo) -> TokenStream2 {
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

fn generate_repository_impl(entity: &EntityInfo) -> TokenStream2 {
    let name = &entity.name;
    let db_fields = entity.db_fields();

    // SQL strings
    let insert_sql = generate_insert_sql(entity);
    let upsert_sql = generate_upsert_sql(entity);
    let update_sql = generate_update_sql(entity);
    let select_by_id_sql = generate_select_by_id_sql(entity);
    let delete_sql = generate_delete_sql(entity);
    let count_sql = generate_count_sql(entity);
    let exists_sql = generate_exists_sql(entity);
    let find_page_sql = generate_find_page_sql(entity);

    // Bind expressions for insert (all db fields)
    let insert_binds: Vec<_> = db_fields.iter().map(|f| generate_bind_value(f)).collect();

    // Bind expressions for update (non-pk first, then pk)
    let non_pk_fields = entity.non_pk_fields();
    let pk_fields = entity.pk_fields();
    let update_binds: Vec<_> = non_pk_fields
        .iter()
        .chain(pk_fields.iter())
        .map(|f| generate_bind_value(f))
        .collect();

    // PK bind for single lookups
    let pk_bind = generate_pk_bind(entity);

    // from_row implementation
    let from_row = generate_from_row(entity);

    // Columns for find_by_ids
    let columns = generate_columns(entity);
    let columns_str = columns.join(", ");
    let table = &entity.table;
    let pk_columns = generate_pk_columns(entity);

    // Generate find_by_ids body based on single vs composite key
    let find_by_ids_body = if pk_fields.len() == 1 {
        let pk_col = &pk_columns[0];
        quote! {
            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let placeholders: Vec<String> = (1..=ids.len())
                .map(|i| format!("${}", i))
                .collect();

            let sql = format!(
                "SELECT {} FROM {} WHERE {} IN ({})",
                #columns_str,
                #table,
                #pk_col,
                placeholders.join(", ")
            );

            let mut query = gearbox_rs_postgres::query(&sql);
            for id in ids {
                query = query.bind(id);
            }

            let rows = query.fetch_all(self.pool.as_ref()).await?;
            Ok(rows.into_iter().map(|row| #from_row).collect())
        }
    } else {
        // Composite key - use sequential lookups (simpler and correct)
        quote! {
            if ids.is_empty() {
                return Ok(Vec::new());
            }

            let mut results = Vec::with_capacity(ids.len());
            for id in ids {
                if let Some(entity) = <Self as gearbox_rs_postgres::PgRepository<#name>>::find_by_id(self, id).await? {
                    results.push(entity);
                }
            }
            Ok(results)
        }
    };

    // Generate delete_batch body
    let delete_batch_body = if pk_fields.len() == 1 {
        let pk_col = &pk_columns[0];
        quote! {
            if ids.is_empty() {
                return Ok(0);
            }

            let placeholders: Vec<String> = (1..=ids.len())
                .map(|i| format!("${}", i))
                .collect();

            let sql = format!(
                "DELETE FROM {} WHERE {} IN ({})",
                #table,
                #pk_col,
                placeholders.join(", ")
            );

            let mut query = gearbox_rs_postgres::query(&sql);
            for id in ids {
                query = query.bind(id);
            }

            let result = query.execute(self.pool.as_ref()).await?;
            Ok(result.rows_affected())
        }
    } else {
        quote! {
            if ids.is_empty() {
                return Ok(0);
            }

            let mut total = 0u64;
            for id in ids {
                if <Self as gearbox_rs_postgres::PgRepository<#name>>::delete(self, id).await? {
                    total += 1;
                }
            }
            Ok(total)
        }
    };

    quote! {
        #[gearbox_rs_core::async_trait]
        impl gearbox_rs_postgres::PgRepository<#name> for gearbox_rs_postgres::PgClient {
            async fn create(&self, entity: #name) -> Result<#name, gearbox_rs_postgres::PgError> {
                gearbox_rs_postgres::query(#insert_sql)
                    #(.bind(#insert_binds))*
                    .execute(self.pool.as_ref())
                    .await?;
                Ok(entity)
            }

            async fn upsert(&self, entity: #name) -> Result<#name, gearbox_rs_postgres::PgError> {
                gearbox_rs_postgres::query(#upsert_sql)
                    #(.bind(#insert_binds))*
                    .execute(self.pool.as_ref())
                    .await?;
                Ok(entity)
            }

            async fn update(&self, entity: #name) -> Result<#name, gearbox_rs_postgres::PgError> {
                let result = gearbox_rs_postgres::query(#update_sql)
                    #(.bind(#update_binds))*
                    .execute(self.pool.as_ref())
                    .await?;

                if result.rows_affected() == 0 {
                    return Err(gearbox_rs_postgres::PgError::NotFound);
                }
                Ok(entity)
            }

            async fn find_by_id(&self, id: &<#name as gearbox_rs_postgres::PgEntity>::Id) -> Result<Option<#name>, gearbox_rs_postgres::PgError> {
                let row = gearbox_rs_postgres::query(#select_by_id_sql)
                    #pk_bind
                    .fetch_optional(self.pool.as_ref())
                    .await?;

                Ok(row.map(|row| #from_row))
            }

            async fn find_by_ids(&self, ids: &[<#name as gearbox_rs_postgres::PgEntity>::Id]) -> Result<Vec<#name>, gearbox_rs_postgres::PgError> {
                #find_by_ids_body
            }

            async fn find_page(&self, limit: i64, offset: i64) -> Result<Vec<#name>, gearbox_rs_postgres::PgError> {
                let rows = gearbox_rs_postgres::query(#find_page_sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(self.pool.as_ref())
                    .await?;

                Ok(rows.into_iter().map(|row| #from_row).collect())
            }

            async fn exists(&self, id: &<#name as gearbox_rs_postgres::PgEntity>::Id) -> Result<bool, gearbox_rs_postgres::PgError> {
                let row = gearbox_rs_postgres::query(#exists_sql)
                    #pk_bind
                    .fetch_one(self.pool.as_ref())
                    .await?;

                Ok(gearbox_rs_postgres::Row::get::<bool, _>(&row, 0))
            }

            async fn count(&self) -> Result<i64, gearbox_rs_postgres::PgError> {
                let row = gearbox_rs_postgres::query(#count_sql)
                    .fetch_one(self.pool.as_ref())
                    .await?;

                Ok(gearbox_rs_postgres::Row::get::<i64, _>(&row, 0))
            }

            async fn delete(&self, id: &<#name as gearbox_rs_postgres::PgEntity>::Id) -> Result<bool, gearbox_rs_postgres::PgError> {
                let result = gearbox_rs_postgres::query(#delete_sql)
                    #pk_bind
                    .execute(self.pool.as_ref())
                    .await?;

                Ok(result.rows_affected() > 0)
            }

            async fn delete_batch(&self, ids: &[<#name as gearbox_rs_postgres::PgEntity>::Id]) -> Result<u64, gearbox_rs_postgres::PgError> {
                #delete_batch_body
            }

            async fn create_batch(&self, entities: Vec<#name>) -> Result<Vec<#name>, gearbox_rs_postgres::PgError> {
                if entities.is_empty() {
                    return Ok(Vec::new());
                }

                // Use a transaction for batch insert
                for entity in &entities {
                    gearbox_rs_postgres::query(#insert_sql)
                        #(.bind(#insert_binds.clone()))*
                        .execute(self.pool.as_ref())
                        .await?;
                }

                Ok(entities)
            }

            async fn upsert_batch(&self, entities: Vec<#name>) -> Result<Vec<#name>, gearbox_rs_postgres::PgError> {
                if entities.is_empty() {
                    return Ok(Vec::new());
                }

                for entity in &entities {
                    gearbox_rs_postgres::query(#upsert_sql)
                        #(.bind(#insert_binds.clone()))*
                        .execute(self.pool.as_ref())
                        .await?;
                }

                Ok(entities)
            }
        }
    }
}

pub fn generate_pg_entity(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let entity = parse_entity(&input);

    let name = &entity.name;
    let id_type = generate_id_type(&entity);
    let id_fn = generate_id_fn(&entity);
    let from_row = generate_from_row(&entity);

    let columns: Vec<_> = generate_columns(&entity);
    let pk_columns: Vec<_> = generate_pk_columns(&entity);
    let table = &entity.table;

    let entity_impl = quote! {
        impl gearbox_rs_postgres::PgEntity for #name {
            type Id = #id_type;

            const TABLE: &'static str = #table;
            const COLUMNS: &'static [&'static str] = &[#(#columns),*];
            const PK_COLUMNS: &'static [&'static str] = &[#(#pk_columns),*];

            fn id(&self) -> Self::Id {
                #id_fn
            }

            fn from_row(row: gearbox_rs_postgres::PgRow) -> Self {
                #from_row
            }
        }
    };

    let repository_impl = generate_repository_impl(&entity);

    let expanded = quote! {
        #entity_impl
        #repository_impl
    };

    expanded.into()
}
