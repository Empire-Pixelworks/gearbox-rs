use syn::{Ident, Type};

/// Validate that the number of parameters matches the SQL placeholders.
pub(super) fn validate_placeholders(
    name: &Ident,
    params: &[(Ident, Type)],
    sql: &str,
) -> syn::Result<()> {
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
