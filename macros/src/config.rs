use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, LitStr};

pub fn generate_cog_config(attr: TokenStream, item: TokenStream) -> TokenStream {
    let config_key = parse_macro_input!(attr as LitStr);
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;

    quote! {
        #input

        impl gearbox_core::CogConfig for #struct_name {
            const CONFIG_KEY: &'static str = #config_key;
        }
    }
    .into()
}
