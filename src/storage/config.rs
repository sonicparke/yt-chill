//! Configuration management

use crate::error::Result;
use crate::types::Config;
use crate::utils::paths::{ensure_dir, get_config_dir, get_config_path};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

/// Load configuration from file, merging with defaults
pub async fn load_config() -> Result<Config> {
    let config_path = get_config_path();

    if !Path::new(&config_path).exists() {
        return Ok(Config::default());
    }

    let content = fs::read_to_string(&config_path).await?;
    let user_config: Config = serde_json::from_str(&content)?;

    // Merge with defaults (user config takes precedence)
    let mut config = Config::default();
    config.limit = user_config.limit;
    config.video_mode = user_config.video_mode;
    config.max_history_entries = user_config.max_history_entries;
    config.editor = user_config.editor;
    config.player = user_config.player;
    config.selector = user_config.selector;
    config.notify = user_config.notify;

    // Set download_dir with default if empty
    config.download_dir = if user_config.download_dir.is_empty() {
        dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~/Downloads".into())
    } else {
        user_config.download_dir
    };

    Ok(config)
}

/// Save configuration to file
pub async fn save_config(config: &Config) -> Result<()> {
    ensure_dir(&get_config_dir()).await?;
    let content = serde_json::to_string_pretty(config)?;
    fs::write(get_config_path(), content).await?;
    Ok(())
}

/// Open config file in editor
pub async fn edit_config(editor: &str) -> Result<()> {
    let config_path = get_config_path();

    // Ensure config file exists
    if !Path::new(&config_path).exists() {
        save_config(&Config::default()).await?;
    }

    Command::new(editor)
        .arg(&config_path)
        .status()
        .await?;

    Ok(())
}
