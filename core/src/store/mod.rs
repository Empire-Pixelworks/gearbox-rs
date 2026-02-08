use async_trait::async_trait;
use serde::de::DeserializeOwned;
use crate::store::query::QueryBuilder;
use types::{StoreError, Page};
use entity::Entity;

pub mod query;
pub mod types;
pub mod entity;
pub mod value;

#[async_trait]
pub trait Store<E: Entity>: Send + Sync {
    type QueryBuilder: QueryBuilder;
    type Pagination: DeserializeOwned + Send;

    async fn get(&self, id: &E::Id) -> Result<Option<E>, StoreError>;
    async fn insert(&self, entity: E) -> Result<E, StoreError>;
    async fn update(&self, entity: E) -> Result<E, StoreError>;
    async fn delete(&self, id: &E::Id) -> Result<bool, StoreError>;

    fn query_builder(&self) -> Self::QueryBuilder;
    async fn execute_query(&self, query: <Self::QueryBuilder as QueryBuilder>::Output) -> Result<Page<E>, StoreError>;
}