//! Configuration management

use crate::error::Result;
use crate::types::Config;
use crate::utils::paths::{ensure_dir, get_config_dir, get_config_path};
use serde_json::json;
use std::path::Path;
use tokio::fs;
use tokio::process::Command;

/// Map legacy `selector` string values removed from [`crate::types::SelectorType`].
fn normalize_selector_in_json(value: &mut serde_json::Value) {
    if let Some(sel) = value.get_mut("selector")
        && let Some(s) = sel.as_str()
        && s.eq_ignore_ascii_case("rofi")
    {
        *sel = json!("dialoguer");
    }
}

/// Load configuration from file, merging with defaults.
///
/// Missing or extra fields in the user's config are tolerated via
/// `#[serde(default)]` on `Config`.
pub async fn load_config() -> Result<Config> {
    let config_path = get_config_path();

    let mut config = if Path::new(&config_path).exists() {
        let content = fs::read_to_string(&config_path).await?;
        let mut raw: serde_json::Value = serde_json::from_str(&content)?;
        normalize_selector_in_json(&mut raw);
        serde_json::from_value(raw)?
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

    Command::new(editor).arg(&config_path).status().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::Result;
    use crate::types::SelectorType;

    #[test]
    fn legacy_rofi_selector_maps_to_dialoguer() -> Result<()> {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"selector": "rofi", "limit": 7}"#).unwrap();
        normalize_selector_in_json(&mut v);
        let cfg: Config = serde_json::from_value(v)?;
        assert_eq!(cfg.selector, SelectorType::Dialoguer);
        assert_eq!(cfg.limit, 7);
        Ok(())
    }

    #[test]
    fn legacy_rofi_uppercase_maps_to_dialoguer() -> Result<()> {
        let mut v: serde_json::Value = serde_json::from_str(r#"{"selector": "Rofi"}"#).unwrap();
        normalize_selector_in_json(&mut v);
        let cfg: Config = serde_json::from_value(v)?;
        assert_eq!(cfg.selector, SelectorType::Dialoguer);
        Ok(())
    }

    #[test]
    fn legacy_notify_field_is_ignored() -> Result<()> {
        let cfg: Config = serde_json::from_str(
            r#"{"notify": true, "limit": 3, "selector": "fzf", "player": "mpv"}"#,
        )?;
        assert_eq!(cfg.limit, 3);
        Ok(())
    }
}
