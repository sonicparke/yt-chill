//! Subscription management
#![allow(dead_code)]

use crate::error::Result;
use crate::types::Subscription;
use crate::utils::paths::{ensure_dir, get_config_dir};
use std::path::PathBuf;
use tokio::fs;

/// Get subscriptions file path
fn get_subscriptions_path() -> PathBuf {
    PathBuf::from(get_config_dir()).join("subscriptions.txt")
}

/// Load subscriptions from file
pub async fn load_subscriptions() -> Result<Vec<Subscription>> {
    let path = get_subscriptions_path();

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path).await?;
    let subscriptions = content
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
        .collect();

    Ok(subscriptions)
}

/// Save subscriptions to file
pub async fn save_subscriptions(subscriptions: &[Subscription]) -> Result<()> {
    ensure_dir(&get_config_dir()).await?;

    let content: String = subscriptions
        .iter()
        .map(|s| format!("{}\t{}", s.name, s.handle))
        .collect::<Vec<_>>()
        .join("\n");

    fs::write(get_subscriptions_path(), content).await?;
    Ok(())
}

/// Add a subscription
pub async fn add_subscription(subscription: &Subscription) -> Result<()> {
    let mut subs = load_subscriptions().await?;

    // Remove existing with same handle
    subs.retain(|s| s.handle != subscription.handle);

    // Add new
    subs.push(subscription.clone());

    save_subscriptions(&subs).await
}

/// Remove a subscription by handle
pub async fn remove_subscription(handle: &str) -> Result<()> {
    let mut subs = load_subscriptions().await?;
    subs.retain(|s| s.handle != handle);
    save_subscriptions(&subs).await
}
