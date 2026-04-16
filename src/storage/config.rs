//! Configuration management

use crate::error::Result;
use crate::types::Config;
use crate::utils::paths::{ensure_dir, get_config_dir, get_config_path};
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

/// Load configuration from file, merging with defaults.
///
/// Missing or extra fields in the user's config are tolerated via
/// `#[serde(default)]` on `Config`.
pub async fn load_config() -> Result<Config> {
    let config_path = get_config_path();

    let mut config = if Path::new(&config_path).exists() {
        let content = fs::read_to_string(&config_path).await?;
        serde_json::from_str::<Config>(&content)?
    } else {
        Config::default()
    };

    config.download_dir = resolve_download_dir(&config.download_dir);

    Ok(config)
}

/// Resolve `download_dir`: fall back to the platform default when empty,
/// then expand any leading `~`.
fn resolve_download_dir(raw: &str) -> String {
    let base = if raw.is_empty() {
        dirs::download_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| "~/Downloads".into())
    } else {
        raw.to_string()
    };

    shellexpand::tilde(&base).into_owned()
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
