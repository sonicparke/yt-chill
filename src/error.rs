//! Error types for yt-chill

use thiserror::Error;

/// Main error type for yt-chill
#[derive(Error, Debug)]
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
    #[allow(dead_code)]
    NoSelection,

    #[error("Invalid configuration: {0}")]
    #[allow(dead_code)]
    InvalidConfig(String),

    #[error("File error: {0}")]
    File(#[from] std::io::Error),

    #[error("Failed to spawn process: {0}")]
    Spawn(String),

    #[error("Playback failed: {0}")]
    Playback(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, YtChillError>;
