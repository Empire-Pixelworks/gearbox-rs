use thiserror::Error;

#[derive(Debug, Error)]
#[non_exhaustive]
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

    #[error("Config not found: {0}")]
    ConfigNotFound(String),

    #[error("Config deserialization failed for '{0}'")]
    ConfigDeserialize(String, #[source] Box<dyn std::error::Error + Send + Sync>),

    #[error("Config validation failed for '{0}': {1}")]
    ConfigValidation(String, String),

    #[error("Config loading failed")]
    ConfigLoad(#[from] config::ConfigError),

    #[error("Failed to bind to {0}")]
    TcpBind(String, #[source] std::io::Error),

    #[error("HTTP server error")]
    Serve(#[source] std::io::Error),

    #[error("{context}")]
    External {
        context: String,
        #[source]
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("Server error: {0}")]
    ServerError(String),
}
