use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, parse_macro_input};

pub fn generate_gearbox_app(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let fn_name = &input.sig.ident;

    // Verify function name is "main"
    if fn_name != "main" {
        return syn::Error::new_spanned(fn_name, "#[gearbox_app] can only be applied to fn main()")
            .to_compile_error()
            .into();
    }

    let core = crate::paths::core_crate();

    let expanded = quote! {
        #[tokio::main]
        async fn main() -> Result<(), #core::Error> {
            #core::Gearbox::crank().await?.ignite().await
        }
    };

    expanded.into()
}
