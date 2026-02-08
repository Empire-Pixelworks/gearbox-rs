use thiserror::Error;

pub struct Page<T> {
    pub items: Vec<T>,
    pub total: Option<u64>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Error, Debug)]
pub enum StoreError {
    #[error("Invalid Query: {0}")]
    InvalidQuery(String),
    #[error("Operation not supported: {0}")]
    UnsupportedOperation(String),
    #[error("There was an error {0}")]
    InternalError(String),
}