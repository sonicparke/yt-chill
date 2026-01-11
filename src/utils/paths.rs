//! Path utilities for yt-chill
//!
//! Respects XDG Base Directory Specification

use crate::error::Result;
use std::env;
use tokio::fs;

const APP_NAME: &str = "yt-chill";

/// Get config directory path
/// Respects XDG_CONFIG_HOME, defaults to ~/.config/yt-chill
pub fn get_config_dir() -> String {
    let base = env::var("XDG_CONFIG_HOME")
        .unwrap_or_else(|_| {
            dirs::config_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}/.config", env::var("HOME").unwrap_or_default()))
        });

    format!("{}/{}", base, APP_NAME)
}

/// Get cache directory path
/// Respects XDG_CACHE_HOME, defaults to ~/.cache/yt-chill
pub fn get_cache_dir() -> String {
    let base = env::var("XDG_CACHE_HOME")
        .unwrap_or_else(|_| {
            dirs::cache_dir()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("{}/.cache", env::var("HOME").unwrap_or_default()))
        });

    format!("{}/{}", base, APP_NAME)
}

/// Get history file path
pub fn get_history_path() -> String {
    format!("{}/history.json", get_cache_dir())
}

/// Get config file path
pub fn get_config_path() -> String {
    format!("{}/config.json", get_config_dir())
}

/// Get subscriptions file path
#[allow(dead_code)]
pub fn get_subscriptions_path() -> String {
    format!("{}/subscriptions.txt", get_config_dir())
}

/// Ensure a directory exists
pub async fn ensure_dir(path: &str) -> Result<()> {
    fs::create_dir_all(path).await?;
    Ok(())
}

/// Ensure all required app directories exist
pub async fn ensure_app_dirs() -> Result<()> {
    ensure_dir(&get_config_dir()).await?;
    ensure_dir(&get_cache_dir()).await?;
    Ok(())
}
