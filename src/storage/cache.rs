//! Search result caching
#![allow(dead_code)]

use crate::error::Result;
use crate::types::CacheEntry;
use crate::utils::paths::{ensure_dir, get_cache_dir};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;
use chrono::Utc;

const DEFAULT_TTL: u64 = 3600; // 1 hour

/// Generate cache key from query
pub fn get_cache_key(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Get cache file path
fn cache_path(key: &str) -> PathBuf {
    PathBuf::from(get_cache_dir()).join(format!("{}.json", key))
}

/// Get cached data if valid
pub async fn get_cached<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let path = cache_path(key);

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).await.ok()?;
    let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;

    // Check if expired
    let now = Utc::now().timestamp();
    if now - entry.timestamp > entry.ttl as i64 {
        // Expired, remove cache file
        let _ = fs::remove_file(&path).await;
        return None;
    }

    Some(entry.data)
}

/// Set cache data
pub async fn set_cache<T: serde::Serialize>(key: &str, data: &T) -> Result<()> {
    ensure_dir(&get_cache_dir()).await?;

    let entry = CacheEntry {
        data,
        timestamp: Utc::now().timestamp(),
        ttl: DEFAULT_TTL,
    };

    let content = serde_json::to_string(&entry)?;
    fs::write(cache_path(key), content).await?;
    Ok(())
}

/// Clear all cache
pub async fn clear_cache() -> Result<()> {
    let cache_dir = get_cache_dir();
    if PathBuf::from(&cache_dir).exists() {
        fs::remove_dir_all(&cache_dir).await?;
    }
    Ok(())
}
