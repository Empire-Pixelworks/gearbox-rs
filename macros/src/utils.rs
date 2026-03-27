use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Field, Fields, GenericArgument, PathArguments, Type, punctuated::Punctuated, token::Comma};

/// Extract the inner type `T` from `Arc<T>`.
///
/// Returns `None` if the type is not a `Path` ending in `Arc` with
/// a single angle-bracketed type argument.
pub fn extract_arc_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Arc"
        && let PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(GenericArgument::Type(inner)) = args.args.first()
    {
        return Some(inner);
    }
    None
}

/// Check whether a field has a marker attribute with the given name.
pub fn has_attr(field: &Field, name: &str) -> bool {
    field.attrs.iter().any(|a| a.path().is_ident(name))
}

/// Parse the argument of a field attribute as type `T`.
///
/// Returns `None` if the attribute is absent or parsing fails.
pub fn get_attr_value<T: syn::parse::Parse>(field: &Field, name: &str) -> Option<T> {
    field
        .attrs
        .iter()
        .find(|a| a.path().is_ident(name))
        .and_then(|a| a.parse_args::<T>().ok())
}

/// Strip custom cog attributes from struct fields, keeping only standard Rust attributes.
pub fn strip_custom_attrs(fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(named) => {
            let cleaned: Punctuated<TokenStream2, Comma> = named
                .named
                .iter()
                .map(|f| {
                    let attrs: Vec<_> = f
                        .attrs
                        .iter()
                        .filter(|a| {
                            !a.path().is_ident("inject")
                                && !a.path().is_ident("config")
                                && !a.path().is_ident("default")
                                && !a.path().is_ident("default_async")
                        })
                        .collect();
                    let vis = &f.vis;
                    let name = &f.ident;
                    let ty = &f.ty;
                    quote! { #(#attrs)* #vis #name: #ty }
                })
                .collect();
            quote! { #cleaned }
        }
        _ => quote! {},
    }
}
