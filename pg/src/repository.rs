use crate::{PgEntity, PgError};

/// Repository trait providing CRUD operations for entities on PgClient.
///
/// This trait is implemented for `PgClient` automatically when using `#[derive(PgEntity)]`.
/// Each entity type gets its own implementation, allowing a single `PgClient` to work
/// with multiple entity types.
///
/// # Example
///
/// ```ignore
/// use gearbox_rs_postgres::{PgClient, PgRepository};
///
/// let client: Arc<PgClient> = hub.get()?;
///
/// // Create
/// let user = client.create(user).await?;
///
/// // Read
/// let found = client.find_by_id::<User>(&id).await?;
///
/// // Update
/// let updated = client.update(user).await?;
///
/// // Delete
/// client.delete::<User>(&id).await?;
/// ```
#[gearbox_rs_core::async_trait]
pub trait PgRepository<T: PgEntity>: Send + Sync {
    async fn create(&self, entity: T) -> Result<T, PgError>;
    async fn upsert(&self, entity: T) -> Result<T, PgError>;
    async fn update(&self, entity: T) -> Result<T, PgError>;
    async fn find_by_id(&self, id: &T::Id) -> Result<Option<T>, PgError>;
    async fn find_by_ids(&self, ids: &[T::Id]) -> Result<Vec<T>, PgError>;
    async fn find_page(&self, limit: i64, offset: i64) -> Result<Vec<T>, PgError>;
    async fn exists(&self, id: &T::Id) -> Result<bool, PgError>;
    async fn count(&self) -> Result<i64, PgError>;
    async fn delete(&self, id: &T::Id) -> Result<bool, PgError>;
    async fn delete_batch(&self, ids: &[T::Id]) -> Result<u64, PgError>;
    async fn create_batch(&self, entities: Vec<T>) -> Result<Vec<T>, PgError>;
    async fn upsert_batch(&self, entities: Vec<T>) -> Result<Vec<T>, PgError>;
}
