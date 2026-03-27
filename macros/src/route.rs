use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{FnArg, ItemFn, LitStr, Pat, parse_macro_input};

pub fn generate_route(method: &str, attr: TokenStream, item: TokenStream) -> TokenStream {
    let path = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as ItemFn);

    let core = crate::paths::core_crate();

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
        .map(|arg| transform_param(arg, &core))
        .collect();

    quote! {
        #vis #asyncness fn #handler_name(
            #(#transformed_params),*
        ) #output {
            #body
        }

        #core::inventory::submit!(#core::RouteRegistration {
            path: #path,
            method: #method,
            handler: || axum::routing::#method_ident(#handler_name),
        });
    }
    .into()
}

fn transform_param(arg: &FnArg, core: &TokenStream2) -> TokenStream2 {
    match arg {
        FnArg::Typed(pat_type) => {
            let pat = &pat_type.pat;
            let ty = &pat_type.ty;

            if let Some(inner) = crate::utils::extract_arc_inner(ty) {
                let new_pat = match pat.as_ref() {
                    Pat::Ident(ident) => {
                        let name = &ident.ident;
                        quote! { #core::Inject(#name) }
                    }
                    _ => quote! { #pat },
                };
                quote! { #new_pat: #core::Inject<#inner> }
            } else {
                quote! { #pat: #ty }
            }
        }
        FnArg::Receiver(_) => quote! { #arg },
    }
}
