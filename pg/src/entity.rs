use sqlx::postgres::PgRow;

/// Trait that defines entity metadata for database operations.
///
/// This trait is typically derived using `#[derive(PgEntity)]` from gearbox-rs-macros.
///
/// # Example
///
/// ```ignore
/// use gearbox_rs_macros::PgEntity;
///
/// #[derive(PgEntity)]
/// #[table("users")]
/// pub struct User {
///     #[primary_key]
///     pub id: String,
///     pub name: String,
///     pub email: String,
/// }
/// ```
pub trait PgEntity: Send + Sync + Sized {
    /// The primary key type.
    /// - Single field: that field's type (e.g., `String`, `i64`, `Uuid`)
    /// - Multiple fields: tuple of types (e.g., `(i64, i64)`, `(Uuid, String)`)
    type Id: Send + Sync;

    /// The database table name (may include schema, e.g., "public.users")
    const TABLE: &'static str;

    /// The schema key used to resolve the correct connection pool.
    /// Defaults to "default" for backward compatibility with single-schema configs.
    const SCHEMA: &'static str = "default";

    /// All column names that map to struct fields (excludes `#[skip]` fields)
    const COLUMNS: &'static [&'static str];

    /// Primary key column name(s)
    const PK_COLUMNS: &'static [&'static str];

    /// Returns the entity's primary key value
    fn id(&self) -> Self::Id;

    /// Constructs an entity from a database row
    fn from_row(row: PgRow) -> Self;
}
