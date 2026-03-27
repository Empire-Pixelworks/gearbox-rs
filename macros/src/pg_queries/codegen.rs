use proc_macro2::TokenStream as TokenStream2;
use quote::quote;

use super::{QueryDef, ReturnKind};

/// Generate the return type for a trait method
fn generate_return_type(return_kind: &ReturnKind, pg: &TokenStream2) -> TokenStream2 {
    match return_kind {
        ReturnKind::Option(inner_ty) => {
            quote! { Result<Option<#inner_ty>, #pg::PgError> }
        }
        ReturnKind::Vec(inner_ty) => {
            quote! { Result<Vec<#inner_ty>, #pg::PgError> }
        }
        ReturnKind::Single(ty) => quote! { Result<#ty, #pg::PgError> },
        ReturnKind::Scalar(ty) => quote! { Result<#ty, #pg::PgError> },
        ReturnKind::Unit => quote! { Result<(), #pg::PgError> },
        ReturnKind::Bool => quote! { Result<bool, #pg::PgError> },
        ReturnKind::RowsAffected => quote! { Result<u64, #pg::PgError> },
    }
}

/// Generate the trait method signature
pub(super) fn generate_trait_method(query: &QueryDef, pg: &TokenStream2) -> TokenStream2 {
    let name = &query.name;
    let return_type = generate_return_type(&query.return_kind, pg);

    let param_defs: Vec<TokenStream2> = query
        .params
        .iter()
        .map(|(name, ty)| quote! { #name: #ty })
        .collect();

    quote! {
        fn #name(&self, #(#param_defs),*) -> impl std::future::Future<Output = #return_type> + Send;
    }
}

/// Generate the impl method body
pub(super) fn generate_impl_method(query: &QueryDef, schema: &str, pg: &TokenStream2) -> TokenStream2 {
    let name = &query.name;
    let sql = &query.sql;
    let return_type = generate_return_type(&query.return_kind, pg);

    let param_defs: Vec<TokenStream2> = query
        .params
        .iter()
        .map(|(name, ty)| quote! { #name: #ty })
        .collect();

    let bind_calls: Vec<TokenStream2> = query
        .params
        .iter()
        .map(|(name, _)| quote! { .bind(#name) })
        .collect();

    let body = match &query.return_kind {
        ReturnKind::Option(inner_ty) => {
            quote! {
                sqlx::query_as::<_, #inner_ty>(#sql)
                    #(#bind_calls)*
                    .fetch_optional(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)
            }
        }
        ReturnKind::Vec(inner_ty) => {
            quote! {
                sqlx::query_as::<_, #inner_ty>(#sql)
                    #(#bind_calls)*
                    .fetch_all(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)
            }
        }
        ReturnKind::Single(ty) => {
            quote! {
                sqlx::query_as::<_, #ty>(#sql)
                    #(#bind_calls)*
                    .fetch_one(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)
            }
        }
        ReturnKind::Scalar(ty) => {
            quote! {
                sqlx::query_scalar::<_, #ty>(#sql)
                    #(#bind_calls)*
                    .fetch_one(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)
            }
        }
        ReturnKind::Unit => {
            quote! {
                sqlx::query(#sql)
                    #(#bind_calls)*
                    .execute(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)?;
                Ok(())
            }
        }
        ReturnKind::Bool => {
            quote! {
                let result = sqlx::query(#sql)
                    #(#bind_calls)*
                    .execute(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)?;
                Ok(result.rows_affected() > 0)
            }
        }
        ReturnKind::RowsAffected => {
            quote! {
                let result = sqlx::query(#sql)
                    #(#bind_calls)*
                    .execute(self.get_pool(#schema)?)
                    .await
                    .map_err(#pg::PgError::from)?;
                Ok(result.rows_affected())
            }
        }
    };

    quote! {
        fn #name(&self, #(#param_defs),*) -> impl std::future::Future<Output = #return_type> + Send {
            async move {
                #body
            }
        }
    }
}
