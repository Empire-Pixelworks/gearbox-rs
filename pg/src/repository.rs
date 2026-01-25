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
    /// Insert a new entity into the database.
    /// Returns the entity back on success.
    async fn create(&self, entity: T) -> Result<T, PgError>;

    /// Insert or update an entity based on primary key conflict.
    /// Fields marked with `#[skip_upsert]` will not be updated on conflict.
    async fn upsert(&self, entity: T) -> Result<T, PgError>;

    /// Update an existing entity by its primary key.
    /// Returns the entity back on success.
    async fn update(&self, entity: T) -> Result<T, PgError>;

    /// Find an entity by its primary key.
    async fn find_by_id(&self, id: &T::Id) -> Result<Option<T>, PgError>;

    /// Find multiple entities by their primary keys.
    async fn find_by_ids(&self, ids: &[T::Id]) -> Result<Vec<T>, PgError>;

    /// Find a page of entities with limit and offset.
    async fn find_page(&self, limit: i64, offset: i64) -> Result<Vec<T>, PgError>;

    /// Check if an entity exists by its primary key.
    async fn exists(&self, id: &T::Id) -> Result<bool, PgError>;

    /// Count all entities in the table.
    async fn count(&self) -> Result<i64, PgError>;

    /// Delete an entity by its primary key.
    /// Returns true if an entity was deleted, false if it didn't exist.
    async fn delete(&self, id: &T::Id) -> Result<bool, PgError>;

    /// Delete multiple entities by their primary keys.
    /// Returns the count of deleted entities.
    async fn delete_batch(&self, ids: &[T::Id]) -> Result<u64, PgError>;

    /// Insert multiple entities in a single batch operation.
    async fn create_batch(&self, entities: Vec<T>) -> Result<Vec<T>, PgError>;

    /// Insert or update multiple entities in a single batch operation.
    async fn upsert_batch(&self, entities: Vec<T>) -> Result<Vec<T>, PgError>;
}
