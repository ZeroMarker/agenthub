use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentHubError {
    #[error("Catalog error: {0}")]
    CatalogLoadError(String),

    #[error("Installer error: {0}")]
    InstallerError(String),

    #[error("Agent not found: {0}")]
    AgentNotFound(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, AgentHubError>;
