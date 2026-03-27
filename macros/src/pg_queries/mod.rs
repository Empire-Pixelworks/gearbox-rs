mod codegen;
mod parse;
mod validate;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{
    Ident, LitStr, Token, Type,
    parse::{Parse, ParseStream},
    parse_macro_input,
};

use codegen::{generate_impl_method, generate_trait_method};
use parse::parse_query_def;

/// A single query function definition
pub(crate) struct QueryDef {
    name: Ident,
    params: Vec<(Ident, Type)>,
    return_kind: ReturnKind,
    sql: String,
}

/// The different return types we support
pub(crate) enum ReturnKind {
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

/// The full macro input: an optional schema declaration followed by function definitions
struct PgQueriesInput {
    schema: String,
    queries: Vec<QueryDef>,
}

impl Parse for PgQueriesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Parse optional: schema = "name";
        let schema = if input.peek(Ident) && !input.peek(Token![fn]) {
            let ident: Ident = input.parse()?;
            if ident == "schema" {
                input.parse::<Token![=]>()?;
                let lit: LitStr = input.parse()?;
                input.parse::<Token![;]>()?;
                lit.value()
            } else {
                return Err(syn::Error::new_spanned(
                    ident,
                    "Expected 'schema' or 'fn'",
                ));
            }
        } else {
            "default".to_string()
        };

        let mut queries = Vec::new();
        while !input.is_empty() {
            queries.push(parse_query_def(input)?);
        }
        Ok(PgQueriesInput { schema, queries })
    }
}

pub fn pg_queries(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as PgQueriesInput);

    let pg = crate::paths::pg_crate();

    let trait_methods: Vec<TokenStream2> = input
        .queries
        .iter()
        .map(|q| generate_trait_method(q, &pg))
        .collect();

    let schema = &input.schema;
    let impl_methods: Vec<TokenStream2> = input
        .queries
        .iter()
        .map(|q| generate_impl_method(q, schema, &pg))
        .collect();

    let trait_name = if input.schema == "default" {
        quote::format_ident!("PgQueries")
    } else {
        // Convert schema name to PascalCase for the trait name
        let pascal: String = input
            .schema
            .split(['_', '-'])
            .map(|part| {
                let mut chars = part.chars();
                match chars.next() {
                    Some(c) => c.to_uppercase().to_string() + chars.as_str(),
                    None => String::new(),
                }
            })
            .collect();
        quote::format_ident!("PgQueries{}", pascal)
    };

    let expanded = quote! {
        pub trait #trait_name {
            #(#trait_methods)*
        }

        impl #trait_name for #pg::PgClient {
            #(#impl_methods)*
        }
    };

    expanded.into()
}
