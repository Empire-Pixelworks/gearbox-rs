use syn::{Data, DeriveInput, Error, Fields, Ident, Type};

pub(super) struct FieldInfo {
    pub ident: Ident,
    pub ty: Type,
    pub is_primary_key: bool,
    pub skip: bool,
    pub skip_upsert: bool,
    pub pg_type: Option<Type>,
}

pub(super) struct EntityInfo {
    pub name: Ident,
    pub table: String,
    pub schema: Option<String>,
    pub fields: Vec<FieldInfo>,
}

impl EntityInfo {
    pub fn pk_fields(&self) -> Vec<&FieldInfo> {
        self.fields.iter().filter(|f| f.is_primary_key).collect()
    }

    pub fn db_fields(&self) -> Vec<&FieldInfo> {
        self.fields.iter().filter(|f| !f.skip).collect()
    }

    pub fn non_pk_fields(&self) -> Vec<&FieldInfo> {
        self.fields
            .iter()
            .filter(|f| !f.skip && !f.is_primary_key)
            .collect()
    }

    pub fn upsert_update_fields(&self) -> Vec<&FieldInfo> {
        self.fields
            .iter()
            .filter(|f| !f.skip && !f.is_primary_key && !f.skip_upsert)
            .collect()
    }
}

pub(super) fn parse_entity(input: &DeriveInput) -> Result<EntityInfo, Error> {
    let name = input.ident.clone();

    // Parse #[table("name")] attribute
    let table = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("table") {
                attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
            } else {
                None
            }
        })
        .ok_or_else(|| {
            Error::new_spanned(input, "#[table(\"name\")] attribute is required")
        })?;

    let schema = input.attrs.iter().find_map(|attr| {
        if attr.path().is_ident("schema") {
            attr.parse_args::<syn::LitStr>().ok().map(|lit| lit.value())
        } else {
            None
        }
    });

    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => fields
                .named
                .iter()
                .map(|f| {
                    let ident = f
                        .ident
                        .clone()
                        .ok_or_else(|| Error::new_spanned(f, "expected named field"))?;
                    let ty = f.ty.clone();

                    Ok(FieldInfo {
                        ident,
                        ty,
                        is_primary_key: crate::utils::has_attr(f, "primary_key"),
                        skip: crate::utils::has_attr(f, "skip"),
                        skip_upsert: crate::utils::has_attr(f, "skip_upsert"),
                        pg_type: crate::utils::get_attr_value::<Type>(f, "pg_type"),
                    })
                })
                .collect::<Result<Vec<_>, Error>>()?,
            _ => {
                return Err(Error::new_spanned(
                    input,
                    "PgEntity can only be derived for structs with named fields",
                ))
            }
        },
        _ => return Err(Error::new_spanned(input, "PgEntity can only be derived for structs")),
    };

    Ok(EntityInfo {
        name,
        table,
        schema,
        fields,
    })
}
