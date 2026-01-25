use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, FnArg, GenericArgument, ItemFn, LitStr, Pat, PathArguments, Type,
};

pub fn generate_route(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let fn_name = &input.sig.ident;
    let handler_name = format_ident!("__handler_{}", fn_name);
    let vis = &input.vis;
    let body = &input.block;
    let output = &input.sig.output;
    let asyncness = &input.sig.asyncness;

    let method_lower = method.to_lowercase();
    let method_ident = format_ident!("{}", method_lower);

    let transformed_params: Vec<TokenStream2> = input
        .sig
        .inputs
        .iter()
        .map(transform_param)
        .collect();

    quote! {
        #vis #asyncness fn #handler_name(
            #(#transformed_params),*
        ) #output {
            #body
        }

        gearbox_rs_core::inventory::submit!(gearbox_rs_core::RouteRegistration {
            path: #path,
            method: #method,
            handler: || axum::routing::#method_ident(#handler_name),
        });
    }
    .into()
}

fn transform_param(arg: &FnArg) -> TokenStream2 {
    match arg {
        FnArg::Typed(pat_type) => {
            let pat = &pat_type.pat;
            let ty = &pat_type.ty;

            if let Some(inner) = extract_arc_inner(ty) {
                let new_pat = match pat.as_ref() {
                    Pat::Ident(ident) => {
                        let name = &ident.ident;
                        quote! { gearbox_rs_core::Inject(#name) }
                    }
                    _ => quote! { #pat },
                };
                quote! { #new_pat: gearbox_rs_core::Inject<#inner> }
            } else {
                quote! { #pat: #ty }
            }
        }
        FnArg::Receiver(_) => quote! { #arg },
    }
}

fn extract_arc_inner(ty: &Type) -> Option<&Type> {
    if let Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
            && segment.ident == "Arc"
                && let PathArguments::AngleBracketed(args) = &segment.arguments
                    && let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner);
                    }
    None
}
