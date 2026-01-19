use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, Data, DeriveInput, Field, Fields, GenericArgument, PathArguments, Type,
    punctuated::Punctuated, token::Comma,
};

enum FieldKind {
    Inject(Type),
    Config,
    Default,
}

struct ParsedField {
    name: syn::Ident,
    ty: Type,
    kind: FieldKind,
}

pub fn generate_cog(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let struct_name = &input.ident;
    let struct_name_str = struct_name.to_string();

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => panic!("#[cog] only supports structs with named fields"),
        },
        _ => panic!("#[cog] can only be applied to structs"),
    };

    let parsed_fields: Vec<ParsedField> = fields.iter().map(|f| parse_field(f)).collect();

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
                    quote! { let #name: #ty = hub.registry.get::<#inner>()?; }
                }
                FieldKind::Config => {
                    quote! { let #name: #ty = hub.config.get::<#ty>(); }
                }
                FieldKind::Default => {
                    quote! { let #name: #ty = Default::default(); }
                }
            }
        })
        .collect();

    let field_names: Vec<&syn::Ident> = parsed_fields.iter().map(|f| &f.name).collect();

    let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();
    let attrs = &input.attrs;
    let vis = &input.vis;

    let original_struct = match &input.data {
        Data::Struct(data) => {
            let cleaned_fields = strip_custom_attrs(&data.fields);
            quote! {
                #(#attrs)*
                #vis struct #struct_name #ty_generics #where_clause {
                    #cleaned_fields
                }
            }
        }
        _ => unreachable!(),
    };

    quote! {
        #original_struct

        struct #factory_name;

        impl gearbox_core::CogFactory for #factory_name {
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
                hub: std::sync::Arc<gearbox_core::Hub>
            ) -> gearbox_core::BoxFuture<
                'static,
                Result<std::sync::Arc<dyn std::any::Any + Send + Sync>, gearbox_core::Error>
            > {
                Box::pin(async move {
                    Ok(std::sync::Arc::new(
                        <#struct_name #ty_generics as gearbox_core::Cog>::new(hub).await?
                    ) as std::sync::Arc<dyn std::any::Any + Send + Sync>)
                })
            }
        }

        #[gearbox_core::async_trait]
        impl #impl_generics gearbox_core::Cog for #struct_name #ty_generics #where_clause {
            async fn new(
                hub: std::sync::Arc<gearbox_core::Hub>
            ) -> Result<Self, gearbox_core::Error> {
                #(#field_extractions)*
                Ok(Self { #(#field_names),* })
            }
        }

        gearbox_core::inventory::submit!(
            &#factory_name as &'static dyn gearbox_core::CogFactory
        );
    }
    .into()
}

fn parse_field(field: &Field) -> ParsedField {
    let name = field.ident.clone().expect("Field must have a name");
    let ty = field.ty.clone();

    let has_inject = field.attrs.iter().any(|a| a.path().is_ident("inject"));
    let has_config = field.attrs.iter().any(|a| a.path().is_ident("config"));

    let kind = if has_inject {
        let inner = extract_arc_inner(&ty).expect(
            "#[inject] field must be Arc<T>"
        );
        FieldKind::Inject(inner)
    } else if has_config {
        FieldKind::Config
    } else {
        FieldKind::Default
    };

    ParsedField { name, ty, kind }
}

fn extract_arc_inner(ty: &Type) -> Option<Type> {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Arc" {
                if let PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(GenericArgument::Type(inner)) = args.args.first() {
                        return Some(inner.clone());
                    }
                }
            }
        }
    }
    None
}

fn strip_custom_attrs(fields: &Fields) -> TokenStream2 {
    match fields {
        Fields::Named(named) => {
            let cleaned: Punctuated<TokenStream2, Comma> = named
                .named
                .iter()
                .map(|f| {
                    let attrs: Vec<_> = f
                        .attrs
                        .iter()
                        .filter(|a| !a.path().is_ident("inject") && !a.path().is_ident("config"))
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
