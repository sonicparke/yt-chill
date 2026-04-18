//! Path utilities for yt-chill
//!
//! Respects XDG Base Directory Specification on Linux and macOS.
//! On Windows we fall back to `dirs::config_dir()` / `dirs::cache_dir()`
//! because Windows users don't expect XDG-style paths.

use crate::error::Result;
use std::env;
use std::path::PathBuf;
use tokio::fs;

const APP_NAME: &str = "yt-chill";

/// Pure helper that resolves an XDG-style base directory given an
/// already-fetched env var value, a home directory, and the fallback
/// subdirectory (e.g. `.config` or `.cache`).
///
/// Kept as a pure function so it can be unit tested without mutating
/// process-wide environment variables.
fn resolve_xdg_base(env_value: Option<&str>, home: &str, fallback_sub: &str) -> String {
    match env_value {
        Some(v) => v.to_string(),
        None => format!("{}/{}", home, fallback_sub),
    }
}

/// Resolve an XDG-style base directory for the current platform.
///
/// * If `env_var` is set, its value is returned as-is.
/// * On Linux and macOS, falls back to `$HOME/<fallback_sub>`.
/// * On other platforms (Windows), falls back to `dirs_fn()` and then
///   to `$HOME/<fallback_sub>` if that also fails.
///
/// `dirs_fn` is only consulted on non-Linux/macOS targets, but we accept
/// it unconditionally to keep the signature portable.
fn xdg_base_dir(env_var: &str, fallback_sub: &str, dirs_fn: fn() -> Option<PathBuf>) -> String {
    let env_value = env::var(env_var).ok();

    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        // Silence "unused" on unix builds; dirs_fn is only used on other OSes.
        let _ = dirs_fn;
        let home = env::var("HOME").unwrap_or_default();
        resolve_xdg_base(env_value.as_deref(), &home, fallback_sub)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        if let Some(v) = env_value {
            return v;
        }
        let home = env::var("HOME").unwrap_or_default();
        dirs_fn()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| format!("{}/{}", home, fallback_sub))
    }
}

/// Get config directory path
/// Respects XDG_CONFIG_HOME, defaults to ~/.config/yt-chill on Linux/macOS.
pub fn get_config_dir() -> String {
    let base = xdg_base_dir("XDG_CONFIG_HOME", ".config", dirs::config_dir);
    format!("{}/{}", base, APP_NAME)
}

/// Get cache directory path
/// Respects XDG_CACHE_HOME, defaults to ~/.cache/yt-chill on Linux/macOS.
pub fn get_cache_dir() -> String {
    let base = xdg_base_dir("XDG_CACHE_HOME", ".cache", dirs::cache_dir);
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

/// Ensure a directory exists
pub async fn ensure_dir(path: &str) -> Result<()> {
    fs::create_dir_all(path).await?;
    Ok(())
}

/// Ensure all required app directories exist.
///
/// On macOS, also performs a one-time best-effort migration from the old
/// `~/Library/Application Support/yt-chill` and `~/Library/Caches/yt-chill`
/// locations (what `dirs::config_dir()` / `dirs::cache_dir()` previously
/// returned) to the new XDG locations under `~/.config` and `~/.cache`.
pub async fn ensure_app_dirs() -> Result<()> {
    #[cfg(target_os = "macos")]
    migrate_macos_legacy_dirs().await;

    ensure_dir(&get_config_dir()).await?;
    ensure_dir(&get_cache_dir()).await?;
    Ok(())
}

/// One-time best-effort migration from the legacy macOS locations to the
/// new XDG locations. Any failure is logged as a warning and swallowed so
/// it never aborts startup.
#[cfg(target_os = "macos")]
async fn migrate_macos_legacy_dirs() {
    let home = match env::var("HOME") {
        Ok(h) if !h.is_empty() => h,
        _ => return,
    };

    let new_config = get_config_dir();
    let new_cache = get_cache_dir();
    let old_config = format!("{}/Library/Application Support/{}", home, APP_NAME);
    let old_cache = format!("{}/Library/Caches/{}", home, APP_NAME);

    let mut migrated = false;

    for (old, new) in [(old_config, new_config), (old_cache, new_cache)] {
        if fs::metadata(&new).await.is_ok() {
            continue;
        }
        if fs::metadata(&old).await.is_err() {
            continue;
        }

        if let Some(parent) = std::path::Path::new(&new).parent()
            && let Err(e) = fs::create_dir_all(parent).await
        {
            eprintln!(
                "Warning: could not create parent directory {}: {}",
                parent.display(),
                e
            );
            continue;
        }

        match fs::rename(&old, &new).await {
            Ok(()) => migrated = true,
            Err(rename_err) => {
                eprintln!(
                    "Warning: could not rename {} to {} ({}); falling back to copy.",
                    old, new, rename_err
                );
                match copy_dir_recursive(&old, &new).await {
                    Ok(()) => migrated = true,
                    Err(copy_err) => {
                        eprintln!(
                            "Warning: fallback copy from {} to {} failed: {}",
                            old, new, copy_err
                        );
                    }
                }
            }
        }
    }

    if migrated {
        println!("Migrated yt-chill data from ~/Library/Application Support to ~/.config");
    }
}

/// Recursively copy a directory tree. Used as a fallback when `rename`
/// fails (e.g. across filesystems).
#[cfg(target_os = "macos")]
async fn copy_dir_recursive(src: &str, dst: &str) -> std::io::Result<()> {
    fs::create_dir_all(dst).await?;
    let mut entries = fs::read_dir(src).await?;
    while let Some(entry) = entries.next_entry().await? {
        let file_type = entry.file_type().await?;
        let src_path = entry.path();
        let dst_path = std::path::Path::new(dst).join(entry.file_name());
        if file_type.is_dir() {
            let src_str = src_path.to_string_lossy().into_owned();
            let dst_str = dst_path.to_string_lossy().into_owned();
            Box::pin(copy_dir_recursive(&src_str, &dst_str)).await?;
        } else {
            fs::copy(&src_path, &dst_path).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_xdg_base_uses_env_when_set() {
        let out = resolve_xdg_base(Some("/custom/path"), "/home/user", ".config");
        assert_eq!(out, "/custom/path");
    }

    #[test]
    fn resolve_xdg_base_env_takes_precedence_over_home() {
        let out = resolve_xdg_base(Some("/xdg/config"), "/home/user", ".config");
        assert_eq!(out, "/xdg/config");
    }

    #[test]
    fn resolve_xdg_base_falls_back_to_home_config() {
        let out = resolve_xdg_base(None, "/home/user", ".config");
        assert_eq!(out, "/home/user/.config");
    }

    #[test]
    fn resolve_xdg_base_falls_back_to_home_cache() {
        let out = resolve_xdg_base(None, "/home/user", ".cache");
        assert_eq!(out, "/home/user/.cache");
    }

    #[test]
    fn resolve_xdg_base_empty_env_value_is_respected_as_set() {
        // An explicitly set-but-empty env value is treated as "set" to mirror
        // the existing `env::var(...).unwrap_or_else(...)` behavior elsewhere
        // in the codebase; callers that want XDG spec-style empty-as-unset
        // handling should pre-filter the Option.
        let out = resolve_xdg_base(Some(""), "/home/user", ".config");
        assert_eq!(out, "");
    }
}
