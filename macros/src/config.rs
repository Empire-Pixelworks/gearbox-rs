use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::{DeriveInput, LitStr, Token, parse_macro_input};

/// Parsed arguments for `#[cog_config("key")]` or `#[cog_config("key", validate = "fn_name")]`.
struct CogConfigArgs {
    key: LitStr,
    validate_fn: Option<syn::Ident>,
}

impl Parse for CogConfigArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: LitStr = input.parse()?;
        let mut validate_fn = None;

        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
            let ident: syn::Ident = input.parse()?;
            if ident != "validate" {
                return Err(syn::Error::new_spanned(
                    ident,
                    "expected `validate`",
                ));
            }
            let _eq: Token![=] = input.parse()?;
            let fn_name: LitStr = input.parse()?;
            validate_fn = Some(format_ident!("{}", fn_name.value()));
        }

        Ok(CogConfigArgs { key, validate_fn })
    }
}

pub fn generate_cog_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as CogConfigArgs);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;
    let config_key = &args.key;

    let core = crate::paths::core_crate();

    // Generate a unique function name for the type_id getter (snake_case)
    let struct_name_lower = struct_name.to_string().to_lowercase();
    let type_id_fn_name = format_ident!("__config_type_id_{}", struct_name_lower);

    let validate_impl = if let Some(fn_name) = &args.validate_fn {
        quote! {
            fn validate(&self) -> Result<(), String> {
                #fn_name(self)
            }
        }
    } else {
        quote! {}
    };

    quote! {
        #input

        impl #core::CogConfig for #struct_name {
            const CONFIG_KEY: &'static str = #config_key;
            #validate_impl
        }

        // Function to get TypeId at runtime (avoids const fn stability issues)
        fn #type_id_fn_name() -> std::any::TypeId {
            std::any::TypeId::of::<#struct_name>()
        }

        // Register with inventory for auto-discovery at startup
        #core::inventory::submit! {
            #core::ConfigMeta {
                key: #config_key,
                type_id_fn: #type_id_fn_name,
                type_name: stringify!(#struct_name),
                deserialize_fn: #core::deserialize_config::<#struct_name>,
                validate_fn: #core::validate_config::<#struct_name>,
            }
        }
    }
    .into()
}
