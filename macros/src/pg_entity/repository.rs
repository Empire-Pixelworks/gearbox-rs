use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::parse::EntityInfo;
use super::sql::*;

pub(super) fn generate_repository_impl(entity: &EntityInfo, core: &TokenStream2, pg: &TokenStream2) -> TokenStream2 {
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
    let from_row = generate_from_row(entity, pg);

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

            let mut query = #pg::query(&sql);
            for id in ids {
                query = query.bind(id);
            }

            let rows = query.fetch_all(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?).await?;
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
                if let Some(entity) = <Self as #pg::PgRepository<#name>>::find_by_id(self, id).await? {
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

            let mut query = #pg::query(&sql);
            for id in ids {
                query = query.bind(id);
            }

            let result = query.execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?).await?;
            Ok(result.rows_affected())
        }
    } else {
        quote! {
            if ids.is_empty() {
                return Ok(0);
            }

            let mut total = 0u64;
            for id in ids {
                if <Self as #pg::PgRepository<#name>>::delete(self, id).await? {
                    total += 1;
                }
            }
            Ok(total)
        }
    };

    quote! {
        #[#core::async_trait]
        impl #pg::PgRepository<#name> for #pg::PgClient {
            async fn create(&self, entity: #name) -> Result<#name, #pg::PgError> {
                #pg::query(#insert_sql)
                    #(.bind(#insert_binds))*
                    .execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;
                Ok(entity)
            }

            async fn upsert(&self, entity: #name) -> Result<#name, #pg::PgError> {
                #pg::query(#upsert_sql)
                    #(.bind(#insert_binds))*
                    .execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;
                Ok(entity)
            }

            async fn update(&self, entity: #name) -> Result<#name, #pg::PgError> {
                let result = #pg::query(#update_sql)
                    #(.bind(#update_binds))*
                    .execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;

                if result.rows_affected() == 0 {
                    return Err(#pg::PgError::NotFound);
                }
                Ok(entity)
            }

            async fn find_by_id(&self, id: &<#name as #pg::PgEntity>::Id) -> Result<Option<#name>, #pg::PgError> {
                let row = #pg::query(#select_by_id_sql)
                    #pk_bind
                    .fetch_optional(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;

                Ok(row.map(|row| #from_row))
            }

            async fn find_by_ids(&self, ids: &[<#name as #pg::PgEntity>::Id]) -> Result<Vec<#name>, #pg::PgError> {
                #find_by_ids_body
            }

            async fn find_page(&self, limit: i64, offset: i64) -> Result<Vec<#name>, #pg::PgError> {
                let rows = #pg::query(#find_page_sql)
                    .bind(limit)
                    .bind(offset)
                    .fetch_all(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;

                Ok(rows.into_iter().map(|row| #from_row).collect())
            }

            async fn exists(&self, id: &<#name as #pg::PgEntity>::Id) -> Result<bool, #pg::PgError> {
                let row = #pg::query(#exists_sql)
                    #pk_bind
                    .fetch_one(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;

                Ok(#pg::Row::get::<bool, _>(&row, 0))
            }

            async fn count(&self) -> Result<i64, #pg::PgError> {
                let row = #pg::query(#count_sql)
                    .fetch_one(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;

                Ok(#pg::Row::get::<i64, _>(&row, 0))
            }

            async fn delete(&self, id: &<#name as #pg::PgEntity>::Id) -> Result<bool, #pg::PgError> {
                let result = #pg::query(#delete_sql)
                    #pk_bind
                    .execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                    .await?;

                Ok(result.rows_affected() > 0)
            }

            async fn delete_batch(&self, ids: &[<#name as #pg::PgEntity>::Id]) -> Result<u64, #pg::PgError> {
                #delete_batch_body
            }

            async fn create_batch(&self, entities: Vec<#name>) -> Result<Vec<#name>, #pg::PgError> {
                if entities.is_empty() {
                    return Ok(Vec::new());
                }

                // Use a transaction for batch insert
                for entity in &entities {
                    #pg::query(#insert_sql)
                        #(.bind(#insert_binds.clone()))*
                        .execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                        .await?;
                }

                Ok(entities)
            }

            async fn upsert_batch(&self, entities: Vec<#name>) -> Result<Vec<#name>, #pg::PgError> {
                if entities.is_empty() {
                    return Ok(Vec::new());
                }

                for entity in &entities {
                    #pg::query(#upsert_sql)
                        #(.bind(#insert_binds.clone()))*
                        .execute(self.get_pool(<#name as #pg::PgEntity>::SCHEMA)?)
                        .await?;
                }

                Ok(entities)
            }
        }
    }
}
