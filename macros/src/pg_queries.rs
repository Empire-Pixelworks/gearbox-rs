use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Ident, LitStr, Token, Type, braced,
    parse::{Parse, ParseStream},
    parse_macro_input,
    punctuated::Punctuated,
};

/// A single query function definition
struct QueryDef {
    name: Ident,
    params: Vec<(Ident, Type)>,
    return_kind: ReturnKind,
    sql: String,
}

/// The different return types we support
enum ReturnKind {
    /// Option<T> - fetch_optional with query_as
    Option(Type),
    /// Vec<T> - fetch_all with query_as
    Vec(Type),
    /// T (struct) - fetch_one with query_as
    Single(Type),
    /// Scalar type (i64, String, etc.) - fetch_one with query_scalar
    Scalar(Type),
    /// No return type - execute, return ()
    Unit,
    /// bool - execute, return rows_affected > 0
    Bool,
    /// u64 - execute, return rows_affected
    RowsAffected,
}

/// The full macro input: a list of function definitions
struct PgQueriesInput {
    queries: Vec<QueryDef>,
}

impl Parse for PgQueriesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut queries = Vec::new();
        while !input.is_empty() {
            queries.push(parse_query_def(input)?);
        }
        Ok(PgQueriesInput { queries })
    }
}

/// Parse a single query definition
fn parse_query_def(input: ParseStream) -> syn::Result<QueryDef> {
    // Parse: fn name(params) -> ReturnType { "SQL" }
    input.parse::<Token![fn]>()?;
    let name: Ident = input.parse()?;

    // Parse parameters
    let params_content;
    syn::parenthesized!(params_content in input);
    let params = parse_params(&params_content)?;

    // Parse return type (optional)
    let return_kind = if input.peek(Token![->]) {
        input.parse::<Token![->]>()?;
        parse_return_kind(input)?
    } else {
        ReturnKind::Unit
    };

    // Parse the SQL string in braces
    let sql_content;
    braced!(sql_content in input);
    let sql_lit: LitStr = sql_content.parse()?;
    let sql = sql_lit.value();

    // Validate parameter count matches placeholders
    validate_placeholders(&name, &params, &sql)?;

    Ok(QueryDef {
        name,
        params,
        return_kind,
        sql,
    })
}

/// Parse function parameters: `name: Type, name2: Type2`
fn parse_params(input: ParseStream) -> syn::Result<Vec<(Ident, Type)>> {
    if input.is_empty() {
        return Ok(Vec::new());
    }

    let params: Punctuated<(Ident, Type), Token![,]> =
        Punctuated::parse_terminated_with(input, |input| {
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            let ty: Type = input.parse()?;
            Ok((name, ty))
        })?;

    Ok(params.into_iter().collect())
}

/// Parse the return type and categorize it
fn parse_return_kind(input: ParseStream) -> syn::Result<ReturnKind> {
    let ty: Type = input.parse()?;
    let ty_str = quote!(#ty).to_string().replace(' ', "");

    // Check for Option<T>
    if let Some(inner) = extract_generic(&ty_str, "Option") {
        let inner_ty: Type = syn::parse_str(&inner)?;
        return Ok(ReturnKind::Option(inner_ty));
    }

    // Check for Vec<T>
    if let Some(inner) = extract_generic(&ty_str, "Vec") {
        let inner_ty: Type = syn::parse_str(&inner)?;
        return Ok(ReturnKind::Vec(inner_ty));
    }

    // Check for ()
    if ty_str == "()" {
        return Ok(ReturnKind::Unit);
    }

    // Check for bool
    if ty_str == "bool" {
        return Ok(ReturnKind::Bool);
    }

    // Check for u64 (rows affected)
    if ty_str == "u64" {
        return Ok(ReturnKind::RowsAffected);
    }

    // Check for scalar types
    if is_scalar_type(&ty_str) {
        return Ok(ReturnKind::Scalar(ty));
    }

    // Default: assume it's a struct type (Single)
    Ok(ReturnKind::Single(ty))
}

/// Extract the inner type from a generic like `Option<Foo>` -> `Foo`
fn extract_generic(ty_str: &str, wrapper: &str) -> Option<String> {
    let prefix = format!("{}<", wrapper);
    if ty_str.starts_with(&prefix) && ty_str.ends_with('>') {
        Some(ty_str[prefix.len()..ty_str.len() - 1].to_string())
    } else {
        None
    }
}

fn is_scalar_type(ty_str: &str) -> bool {
    matches!(
        ty_str,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u128"
            | "f32"
            | "f64"
            | "String"
            | "&str"
            | "isize"
            | "usize"
    )
}

/// Validate that the number of parameters matches the SQL placeholders
fn validate_placeholders(name: &Ident, params: &[(Ident, Type)], sql: &str) -> syn::Result<()> {
    let mut max_placeholder = 0;
    let mut i = 0;
    let chars: Vec<char> = sql.chars().collect();

    while i < chars.len() {
        if chars[i] == '$' {
            let mut num_str = String::new();
            i += 1;
            while i < chars.len() && chars[i].is_ascii_digit() {
                num_str.push(chars[i]);
                i += 1;
            }
            if let Ok(n) = num_str.parse::<usize>() {
                max_placeholder = max_placeholder.max(n);
            }
        } else {
            i += 1;
        }
    }

    let param_count = params.len();

    if max_placeholder != param_count {
        return Err(syn::Error::new_spanned(
            name,
            format!(
                "Parameter count mismatch in query '{}': found {} parameters but SQL has {} placeholders",
                name, param_count, max_placeholder
            ),
        ));
    }

    Ok(())
}

/// Generate the return type for a trait method
fn generate_return_type(return_kind: &ReturnKind) -> TokenStream2 {
    match return_kind {
        ReturnKind::Option(inner_ty) => quote! { Result<Option<#inner_ty>, gearbox_rs_postgres::PgError> },
        ReturnKind::Vec(inner_ty) => quote! { Result<Vec<#inner_ty>, gearbox_rs_postgres::PgError> },
        ReturnKind::Single(ty) => quote! { Result<#ty, gearbox_rs_postgres::PgError> },
        ReturnKind::Scalar(ty) => quote! { Result<#ty, gearbox_rs_postgres::PgError> },
        ReturnKind::Unit => quote! { Result<(), gearbox_rs_postgres::PgError> },
        ReturnKind::Bool => quote! { Result<bool, gearbox_rs_postgres::PgError> },
        ReturnKind::RowsAffected => quote! { Result<u64, gearbox_rs_postgres::PgError> },
    }
}

/// Generate the trait method signature
fn generate_trait_method(query: &QueryDef) -> TokenStream2 {
    let name = &query.name;
    let return_type = generate_return_type(&query.return_kind);

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
fn generate_impl_method(query: &QueryDef) -> TokenStream2 {
    let name = &query.name;
    let sql = &query.sql;
    let return_type = generate_return_type(&query.return_kind);

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
                    .fetch_optional(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)
            }
        }
        ReturnKind::Vec(inner_ty) => {
            quote! {
                sqlx::query_as::<_, #inner_ty>(#sql)
                    #(#bind_calls)*
                    .fetch_all(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)
            }
        }
        ReturnKind::Single(ty) => {
            quote! {
                sqlx::query_as::<_, #ty>(#sql)
                    #(#bind_calls)*
                    .fetch_one(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)
            }
        }
        ReturnKind::Scalar(ty) => {
            quote! {
                sqlx::query_scalar::<_, #ty>(#sql)
                    #(#bind_calls)*
                    .fetch_one(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)
            }
        }
        ReturnKind::Unit => {
            quote! {
                sqlx::query(#sql)
                    #(#bind_calls)*
                    .execute(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)?;
                Ok(())
            }
        }
        ReturnKind::Bool => {
            quote! {
                let result = sqlx::query(#sql)
                    #(#bind_calls)*
                    .execute(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)?;
                Ok(result.rows_affected() > 0)
            }
        }
        ReturnKind::RowsAffected => {
            quote! {
                let result = sqlx::query(#sql)
                    #(#bind_calls)*
                    .execute(&*self.pool)
                    .await
                    .map_err(gearbox_rs_postgres::PgError::from)?;
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

pub fn pg_queries(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as PgQueriesInput);

    let trait_methods: Vec<TokenStream2> = input
        .queries
        .iter()
        .map(generate_trait_method)
        .collect();

    let impl_methods: Vec<TokenStream2> = input
        .queries
        .iter()
        .map(generate_impl_method)
        .collect();

    let expanded = quote! {
        pub trait PgQueries {
            #(#trait_methods)*
        }

        impl PgQueries for gearbox_rs_postgres::PgClient {
            #(#impl_methods)*
        }
    };

    expanded.into()
}
