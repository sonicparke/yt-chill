//! Configuration management

use crate::error::{Result, YtChillError};
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

    validate_config(&config)?;
    config.download_dir = resolve_download_dir(&config.download_dir);

    Ok(config)
}

fn validate_config(config: &Config) -> Result<()> {
    if config.volume > 100 {
        return Err(YtChillError::InvalidConfig(
            "volume must be between 0 and 100".into(),
        ));
    }

    Ok(())
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
    use crate::core::player::AUDIO_ONLY_FORMAT;
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

    #[test]
    fn legacy_rofi_mixed_case_maps_to_dialoguer() -> Result<()> {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"selector": "RoFi", "player": "mpv"}"#).unwrap();
        normalize_selector_in_json(&mut v);
        let cfg: Config = serde_json::from_value(v)?;
        assert_eq!(cfg.selector, SelectorType::Dialoguer);
        Ok(())
    }

    #[test]
    fn normalize_selector_leaves_fzf_unchanged() -> Result<()> {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"selector": "fzf", "limit": 1}"#).unwrap();
        normalize_selector_in_json(&mut v);
        let cfg: Config = serde_json::from_value(v)?;
        assert_eq!(cfg.selector, SelectorType::Fzf);
        assert_eq!(cfg.limit, 1);
        Ok(())
    }

    #[test]
    fn normalize_selector_ignores_non_string_selector() {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"selector": 42, "limit": 5}"#).unwrap();
        normalize_selector_in_json(&mut v);
        assert_eq!(v["selector"], serde_json::json!(42));
    }

    #[test]
    fn minimal_json_uses_defaults_for_missing_fields() -> Result<()> {
        let cfg: Config = serde_json::from_str(r#"{"limit": 4}"#)?;
        assert_eq!(cfg.limit, 4);
        assert_eq!(cfg.volume, 100);
        assert_eq!(cfg.selector, SelectorType::default());
        assert_eq!(cfg.audio_format.as_deref(), Some(AUDIO_ONLY_FORMAT));
        assert_eq!(cfg.video_format, None);
        Ok(())
    }

    #[test]
    fn config_accepts_app_volume_from_zero_through_one_hundred() -> Result<()> {
        for volume in [0, 50, 100] {
            let cfg: Config = serde_json::from_value(json!({ "volume": volume }))?;
            validate_config(&cfg)?;
            assert_eq!(cfg.volume, volume);
        }
        Ok(())
    }

    #[test]
    fn config_rejects_app_volume_above_one_hundred() {
        let cfg: Config = serde_json::from_value(json!({ "volume": 101 })).unwrap();
        let error = validate_config(&cfg).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("volume must be between 0 and 100")
        );
    }

    #[test]
    fn config_json_accepts_custom_playback_formats() -> Result<()> {
        let cfg: Config = serde_json::from_str(
            r#"{
                "audio_format": "251/bestaudio",
                "video_format": "bestvideo[height<=480]+bestaudio/best"
            }"#,
        )?;

        assert_eq!(cfg.audio_format.as_deref(), Some("251/bestaudio"));
        assert_eq!(
            cfg.video_format.as_deref(),
            Some("bestvideo[height<=480]+bestaudio/best")
        );
        Ok(())
    }

    #[test]
    fn resolve_download_dir_absolute_path_unchanged() {
        assert_eq!(
            resolve_download_dir("/var/tmp/yt-chill-downloads"),
            "/var/tmp/yt-chill-downloads"
        );
    }

    #[test]
    fn resolve_download_dir_empty_uses_default_and_non_empty() {
        let resolved = resolve_download_dir("");
        assert!(!resolved.is_empty());
        assert!(
            !resolved.contains('~'),
            "download dir should be tilde-expanded: {resolved}"
        );
    }

    #[test]
    fn resolve_download_dir_expands_leading_tilde() {
        let resolved = resolve_download_dir("~/Music/yt-chill");
        assert!(
            resolved.ends_with("Music/yt-chill") || resolved.ends_with(r"Music\yt-chill"),
            "unexpected path: {resolved}"
        );
        assert!(!resolved.contains('~'));
    }

    #[test]
    fn normalize_selector_leaves_dialoguer_unchanged() -> Result<()> {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"selector": "dialoguer"}"#).unwrap();
        normalize_selector_in_json(&mut v);
        let cfg: Config = serde_json::from_value(v)?;
        assert_eq!(cfg.selector, SelectorType::Dialoguer);
        Ok(())
    }

    #[test]
    fn normalize_selector_does_not_map_rofi_substring() -> Result<()> {
        let mut v: serde_json::Value =
            serde_json::from_str(r#"{"selector": "rofi-inspired"}"#).unwrap();
        normalize_selector_in_json(&mut v);
        let err = serde_json::from_value::<Config>(v).unwrap_err();
        assert!(
            err.to_string().to_ascii_lowercase().contains("selector")
                || err.to_string().contains("unknown"),
            "unexpected err: {err}"
        );
        Ok(())
    }

    #[test]
    fn normalize_selector_missing_key_is_noop() {
        let mut v: serde_json::Value = serde_json::from_str(r#"{"limit": 2}"#).unwrap();
        let before = v.clone();
        normalize_selector_in_json(&mut v);
        assert_eq!(v, before);
    }
}
