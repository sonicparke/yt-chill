//! Subscription management
//!
//! On-disk format is JSON (`subscriptions.json`). A legacy `subscriptions.txt`
//! TSV file is auto-migrated on first read; the original is renamed to
//! `subscriptions.txt.bak` so the user has a recovery copy.

use crate::error::Result;
use crate::types::Subscription;
use crate::utils::paths::{ensure_dir, get_config_dir};
use std::path::PathBuf;
use tokio::fs;

fn subs_json_path() -> PathBuf {
    PathBuf::from(get_config_dir()).join("subscriptions.json")
}

fn legacy_txt_path() -> PathBuf {
    PathBuf::from(get_config_dir()).join("subscriptions.txt")
}

/// Parse the legacy TSV format: one line per entry, `<name>\t<handle>`.
fn parse_legacy_tsv(content: &str) -> Vec<Subscription> {
    content
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(2, '\t').collect();
            if parts.len() == 2 {
                Some(Subscription {
                    name: parts[0].to_string(),
                    handle: parts[1].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Load subscriptions from disk.
///
/// Preference order:
/// 1. `subscriptions.json` (current format).
/// 2. Legacy `subscriptions.txt` (TSV) — auto-migrated and renamed to `.txt.bak`.
/// 3. Empty list.
pub async fn load_subscriptions() -> Result<Vec<Subscription>> {
    let json_path = subs_json_path();

    if json_path.exists() {
        let content = fs::read_to_string(&json_path).await?;
        return Ok(serde_json::from_str(&content).unwrap_or_default());
    }

    let txt_path = legacy_txt_path();
    if txt_path.exists() {
        let content = fs::read_to_string(&txt_path).await?;
        let subs = parse_legacy_tsv(&content);

        // Best-effort migration: write the new file, then move the old one
        // aside. Failures here are non-fatal — we still return the parsed
        // subscriptions so the user gets working behaviour today.
        if let Err(err) = save_subscriptions(&subs).await {
            eprintln!(
                "Warning: failed to write migrated subscriptions.json: {}",
                err
            );
        } else {
            let bak = txt_path.with_extension("txt.bak");
            if let Err(err) = fs::rename(&txt_path, &bak).await {
                eprintln!(
                    "Warning: migrated subscriptions but failed to rename {} to {}: {}",
                    txt_path.display(),
                    bak.display(),
                    err
                );
            }
        }

        return Ok(subs);
    }

    Ok(Vec::new())
}

/// Save subscriptions to disk as pretty-printed JSON.
pub async fn save_subscriptions(subscriptions: &[Subscription]) -> Result<()> {
    ensure_dir(&get_config_dir()).await?;
    let content = serde_json::to_string_pretty(subscriptions)?;
    fs::write(subs_json_path(), content).await?;
    Ok(())
}

/// Add (or replace, keyed on `handle`) a subscription.
pub async fn add_subscription(subscription: &Subscription) -> Result<()> {
    let mut subs = load_subscriptions().await?;
    subs.retain(|s| s.handle != subscription.handle);
    subs.push(subscription.clone());
    save_subscriptions(&subs).await
}

/// Remove a subscription by handle.
#[allow(dead_code)]
pub async fn remove_subscription(handle: &str) -> Result<()> {
    let mut subs = load_subscriptions().await?;
    subs.retain(|s| s.handle != handle);
    save_subscriptions(&subs).await
}
