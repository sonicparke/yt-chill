//! Error types for yt-chill

use thiserror::Error;

/// Error codes matching the TypeScript implementation
#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    // Network errors
    NetworkError,
    YouTubeParseError,

    // Dependency errors
    MissingDependency,

    // User errors
    NoResults,
    NoSelection,
    InvalidConfig,

    // System errors
    FileError,
    SpawnError,
}

/// Main error type for yt-chill
#[derive(Error, Debug)]
#[allow(dead_code)]
pub enum YtChillError {
    #[error("Network error: {0}")]
    Network(String),

    #[error("Failed to parse YouTube response: {0}")]
    YouTubeParse(String),

    #[error("Missing dependency: {0}. Please install it.")]
    MissingDependency(String),

    #[error("No results found")]
    NoResults,

    #[error("No selection made")]
    NoSelection,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("File error: {0}")]
    File(#[from] std::io::Error),

    #[error("Failed to spawn process: {0}")]
    Spawn(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl YtChillError {
    #[allow(dead_code)]
    pub fn code(&self) -> ErrorCode {
        match self {
            Self::Network(_) => ErrorCode::NetworkError,
            Self::YouTubeParse(_) => ErrorCode::YouTubeParseError,
            Self::MissingDependency(_) => ErrorCode::MissingDependency,
            Self::NoResults => ErrorCode::NoResults,
            Self::NoSelection => ErrorCode::NoSelection,
            Self::InvalidConfig(_) => ErrorCode::InvalidConfig,
            Self::File(_) => ErrorCode::FileError,
            Self::Spawn(_) => ErrorCode::SpawnError,
            Self::Http(_) => ErrorCode::NetworkError,
            Self::Json(_) => ErrorCode::YouTubeParseError,
        }
    }
}

pub type Result<T> = std::result::Result<T, YtChillError>;
