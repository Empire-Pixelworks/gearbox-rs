use quote::quote;
use syn::{Ident, LitStr, Token, Type, braced, parse::ParseStream, punctuated::Punctuated};

use super::{QueryDef, ReturnKind};
use super::validate::validate_placeholders;

/// Parse a single query definition
pub(super) fn parse_query_def(input: ParseStream) -> syn::Result<QueryDef> {
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
