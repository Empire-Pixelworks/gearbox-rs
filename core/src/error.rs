use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Cog not found: {0}")]
    CogNotFound(String),
    #[error("Cog downcast failed: {0}")]
    CogDowncastFailed(String),
    #[error("Cog registration failed: {0}")]
    CogRegisterError(String),
    #[error("Cyclic dependency detected involving: {0}")]
    CyclicDependency(String),
    #[error("Missing dependency: '{0}' requires '{1}' which is not registered")]
    MissingDependency(String, String),
    #[error("Server error: {0}")]
    ServerError(String),
}
