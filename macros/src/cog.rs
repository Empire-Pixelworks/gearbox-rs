use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{Data, DeriveInput, Error, Field, Fields, Type, parse_macro_input};

enum FieldKind {
    Inject(Type),
    Config,
    Default,
    DefaultFn(syn::Path),      // #[default(fn)] -> fn() -> T
    DefaultAsyncFn(syn::Path), // #[default_async(fn)] -> async fn(Arc<Hub>) -> Result<T, Error>
}

struct ParsedField {
    name: syn::Ident,
    ty: Type,
    kind: FieldKind,
}

pub fn generate_cog(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    generate_cog_internal(input).unwrap_or_else(|e| e.to_compile_error().into())
}

fn get_struct_attr_path(attrs: &[syn::Attribute], name: &str) -> Result<Option<syn::Ident>, Error> {
    let attr = attrs.iter().find(|a| a.path().is_ident(name));
    match attr {
        Some(a) => {
            let ident: syn::Ident = a.parse_args().map_err(|_| {
                Error::new_spanned(
                    a,
                    format!("#[{}] requires a method name, e.g. #[{}(my_method)]", name, name),
                )
            })?;
            Ok(Some(ident))
        }
        None => Ok(None),
    }
}

fn strip_lifecycle_attrs(attrs: &[syn::Attribute]) -> Vec<&syn::Attribute> {
    attrs
        .iter()
        .filter(|a| !a.path().is_ident("on_start") && !a.path().is_ident("on_shutdown"))
        .collect()
}

fn generate_cog_internal(input: DeriveInput) -> Result<TokenStream, Error> {
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();

    let on_start_method = get_struct_attr_path(&input.attrs, "on_start")?;
    let on_shutdown_method = get_struct_attr_path(&input.attrs, "on_shutdown")?;

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return Err(Error::new_spanned(
                    &input,
                    "#[cog] only supports structs with named fields",
                ))
            }
        },
        _ => return Err(Error::new_spanned(&input, "#[cog] can only be applied to structs")),
    };

    let parsed_fields: Vec<ParsedField> =
        fields.iter().map(parse_field).collect::<Result<Vec<_>, _>>()?;

    let inject_types: Vec<&Type> = parsed_fields
        .iter()
        .filter_map(|f| match &f.kind {
            FieldKind::Inject(inner) => Some(inner),
            _ => None,
        })
        .collect();

    let factory_name = format_ident!("__{}Factory", struct_name);

    let field_extractions: Vec<TokenStream2> = parsed_fields
        .iter()
        .map(|f| {
            let name = &f.name;
            let ty = &f.ty;
            match &f.kind {
                FieldKind::Inject(inner) => {
                    quote! { let #name: #ty = hub.registry_get::<#inner>()?; }
                }
                FieldKind::Config => {
                    quote! { let #name: #ty = hub.config_get::<#ty>()?; }
                }
                FieldKind::Default => {
                    quote! { let #name: #ty = Default::default(); }
                }
                FieldKind::DefaultFn(fn_path) => {
                    quote! { let #name: #ty = #fn_path(); }
                }
                FieldKind::DefaultAsyncFn(fn_path) => {
                    quote! { let #name: #ty = #fn_path(hub.clone()).await?; }
                }
            }
        })
        .collect();

    let field_names: Vec<&syn::Ident> = parsed_fields.iter().map(|f| &f.name).collect();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let attrs = &input.attrs;
    let vis = &input.vis;

    let clean_attrs = strip_lifecycle_attrs(attrs);

    let original_struct = match &input.data {
        Data::Struct(data) => {
            let cleaned_fields = crate::utils::strip_custom_attrs(&data.fields);
            quote! {
                #(#clean_attrs)*
                #vis struct #struct_name #ty_generics #where_clause {
                    #cleaned_fields
                }
            }
        }
        _ => unreachable!(),
    };

    let core = crate::paths::core_crate();

    let on_start_impl = on_start_method.map(|method| {
        quote! {
            fn on_start(
                &self,
                cog: std::sync::Arc<dyn std::any::Any + Send + Sync>
            ) -> #core::BoxFuture<'static, Result<(), #core::Error>> {
                Box::pin(async move {
                    let concrete = cog.downcast_ref::<#struct_name #ty_generics>()
                        .ok_or_else(|| #core::Error::CogDowncastFailed(#struct_name_str.to_string()))?;
                    concrete.#method().await
                })
            }
        }
    });

    let on_shutdown_impl = on_shutdown_method.map(|method| {
        quote! {
            fn on_shutdown(
                &self,
                cog: std::sync::Arc<dyn std::any::Any + Send + Sync>
            ) -> #core::BoxFuture<'static, Result<(), #core::Error>> {
                Box::pin(async move {
                    let concrete = cog.downcast_ref::<#struct_name #ty_generics>()
                        .ok_or_else(|| #core::Error::CogDowncastFailed(#struct_name_str.to_string()))?;
                    concrete.#method().await
                })
            }
        }
    });

    Ok(quote! {
        #original_struct

        struct #factory_name;

        impl #core::CogFactory for #factory_name {
            fn type_id(&self) -> std::any::TypeId {
                std::any::TypeId::of::<#struct_name #ty_generics>()
            }

            fn type_name(&self) -> &'static str {
                #struct_name_str
            }

            fn deps(&self) -> Vec<std::any::TypeId> {
                vec![#(std::any::TypeId::of::<#inject_types>()),*]
            }

            fn build(
                &self,
                hub: std::sync::Arc<#core::Hub>
            ) -> #core::BoxFuture<
                'static,
                Result<std::sync::Arc<dyn std::any::Any + Send + Sync>, #core::Error>
            > {
                Box::pin(async move {
                    Ok(std::sync::Arc::new(
                        <#struct_name #ty_generics as #core::Cog>::new(hub).await?
                    ) as std::sync::Arc<dyn std::any::Any + Send + Sync>)
                })
            }

            #on_start_impl
            #on_shutdown_impl
        }

        #[#core::async_trait]
        impl #impl_generics #core::Cog for #struct_name #ty_generics #where_clause {
            async fn new(
                hub: std::sync::Arc<#core::Hub>
            ) -> Result<Self, #core::Error> {
                #(#field_extractions)*
                Ok(Self { #(#field_names),* })
            }
        }

        #core::inventory::submit!(
            &#factory_name as &'static dyn #core::CogFactory
        );
    }
    .into())
}

fn parse_field(field: &Field) -> Result<ParsedField, Error> {
    let name = field
        .ident
        .clone()
        .ok_or_else(|| Error::new_spanned(field, "field must have a name"))?;
    let ty = field.ty.clone();

    let has_inject = crate::utils::has_attr(field, "inject");
    let has_config = crate::utils::has_attr(field, "config");

    let default_fn = field
        .attrs
        .iter()
        .find(|a| a.path().is_ident("default"))
        .map(|a| {
            a.parse_args::<syn::Path>().map_err(|_| {
                Error::new_spanned(
                    a,
                    format!(
                        "#[default] on field '{}' requires a function path. \
                        Expected: #[default(my_function)] where my_function: fn() -> {}",
                        name,
                        quote!(#ty)
                    ),
                )
            })
        })
        .transpose()?;

    let default_async_fn = field
        .attrs
        .iter()
        .find(|a| a.path().is_ident("default_async"))
        .map(|a| {
            a.parse_args::<syn::Path>().map_err(|_| {
                Error::new_spanned(
                    a,
                    format!(
                        "#[default_async] on field '{}' requires a function path. \
                        Expected: #[default_async(my_function)] where my_function: \
                        async fn(&Arc<Hub>) -> Result<{}, Error>",
                        name,
                        quote!(#ty)
                    ),
                )
            })
        })
        .transpose()?;

    // Validate no conflicting attributes
    let attr_count = has_inject as u8
        + has_config as u8
        + default_fn.is_some() as u8
        + default_async_fn.is_some() as u8;

    if attr_count > 1 {
        return Err(Error::new_spanned(
            field,
            format!(
                "field '{}' has conflicting attributes. Use only one of: \
                #[inject], #[config], #[default(fn)], #[default_async(fn)]",
                name
            ),
        ));
    }

    let kind = if has_inject {
        let inner = crate::utils::extract_arc_inner(&ty).cloned().ok_or_else(|| {
            Error::new_spanned(&field.ty, format!("#[inject] field '{}' must be Arc<T>", name))
        })?;
        FieldKind::Inject(inner)
    } else if has_config {
        FieldKind::Config
    } else if let Some(fn_path) = default_fn {
        FieldKind::DefaultFn(fn_path)
    } else if let Some(fn_path) = default_async_fn {
        FieldKind::DefaultAsyncFn(fn_path)
    } else {
        FieldKind::Default
    };

    Ok(ParsedField { name, ty, kind })
}

