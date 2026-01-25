use proc_macro::TokenStream;
use quote::{format_ident, quote};
use syn::{parse_macro_input, DeriveInput, LitStr};

pub fn generate_cog_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config_key = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    // Generate a unique function name for the type_id getter (snake_case)
    let struct_name_lower = struct_name.to_string().to_lowercase();
    let type_id_fn_name = format_ident!("__config_type_id_{}", struct_name_lower);

    quote! {
        #input

        impl gearbox_rs_core::CogConfig for #struct_name {
            const CONFIG_KEY: &'static str = #config_key;
        }

        // Function to get TypeId at runtime (avoids const fn stability issues)
        fn #type_id_fn_name() -> std::any::TypeId {
            std::any::TypeId::of::<#struct_name>()
        }

        // Register with inventory for auto-discovery at startup
        gearbox_rs_core::inventory::submit! {
            gearbox_rs_core::ConfigMeta {
                key: #config_key,
                type_id_fn: #type_id_fn_name,
                type_name: stringify!(#struct_name),
                deserialize_fn: gearbox_rs_core::deserialize_config::<#struct_name>,
            }
        }
    }
    .into()
}
