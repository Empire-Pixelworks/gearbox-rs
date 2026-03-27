use proc_macro2::TokenStream;
use proc_macro_crate::{FoundCrate, crate_name};
use quote::quote;

fn found_to_tokens(found: FoundCrate) -> TokenStream {
    match found {
        FoundCrate::Itself => quote! { crate },
        FoundCrate::Name(name) => {
            let ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
            quote! { #ident }
        }
    }
}

/// Resolve a crate path, checking for the `gearbox-rs` facade first,
/// then the direct crate name, then falling back to the given ident.
fn resolve_crate(direct_name: &str, fallback_ident: &str) -> TokenStream {
    if let Ok(found) = crate_name("gearbox-rs") {
        found_to_tokens(found)
    } else if let Ok(found) = crate_name(direct_name) {
        found_to_tokens(found)
    } else {
        let ident = syn::Ident::new(fallback_ident, proc_macro2::Span::call_site());
        quote! { #ident }
    }
}

/// Returns the token path for gearbox-rs-core types.
pub fn core_crate() -> TokenStream {
    resolve_crate("gearbox-rs-core", "gearbox_rs_core")
}

/// Returns the token path for gearbox-rs-postgres types.
pub fn pg_crate() -> TokenStream {
    resolve_crate("gearbox-rs-postgres", "gearbox_rs_postgres")
}
