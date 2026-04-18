//! Search result caching

use crate::error::Result;
use crate::types::CacheEntry;
use crate::utils::paths::{ensure_dir, get_cache_dir};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tokio::fs;

const DEFAULT_TTL: u64 = 3600; // 1 hour

/// Generate cache key from query
pub fn get_cache_key(query: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(query.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Cache key for a video search (`query`, `limit`).
///
/// Matches the legacy `get_cache_key(&format!("video:{}:{}", …))` input so
/// existing cache files stay valid.
pub fn cache_key_video_search(query: &str, limit: usize) -> String {
    get_cache_key(&format!("video:{}:{}", query, limit))
}

/// Cache key for a channel videos fetch (`handle`, `limit`).
///
/// Matches the legacy `get_cache_key(&format!("channel:{}:{}", …))` input.
pub fn cache_key_channel_videos(handle: &str, limit: usize) -> String {
    get_cache_key(&format!("channel:{}:{}", handle, limit))
}

/// Get cache file path
fn cache_path(key: &str) -> PathBuf {
    PathBuf::from(get_cache_dir()).join(format!("{}.json", key))
}

/// Pure TTL check — extracted so it can be unit-tested without touching the
/// filesystem or the clock.
fn is_expired(entry_timestamp: i64, ttl_seconds: u64, now: i64) -> bool {
    now - entry_timestamp > ttl_seconds as i64
}

/// Get cached data if valid
pub async fn get_cached<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
    let path = cache_path(key);

    if !path.exists() {
        return None;
    }

    let content = fs::read_to_string(&path).await.ok()?;
    let entry: CacheEntry<T> = serde_json::from_str(&content).ok()?;

    let now = Utc::now().timestamp();
    if is_expired(entry.timestamp, entry.ttl, now) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_key_is_deterministic() {
        let a = get_cache_key("video:lofi:15");
        let b = get_cache_key("video:lofi:15");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64); // sha256 hex
    }

    #[test]
    fn cache_key_differs_per_input() {
        assert_ne!(get_cache_key("a"), get_cache_key("b"));
    }

    #[test]
    fn cache_key_empty_string_is_stable_hex() {
        let k = get_cache_key("");
        assert_eq!(k.len(), 64);
        assert_eq!(k, get_cache_key(""));
    }

    #[test]
    fn fresh_entry_is_not_expired() {
        let now = 1_000_000i64;
        assert!(!is_expired(now, 3600, now));
        assert!(!is_expired(now - 10, 3600, now));
        assert!(!is_expired(now - 3600, 3600, now));
    }

    #[test]
    fn entry_past_ttl_is_expired() {
        let now = 1_000_000i64;
        assert!(is_expired(now - 3601, 3600, now));
        assert!(is_expired(0, 3600, now));
    }

    #[test]
    fn entry_with_zero_ttl_expires_immediately() {
        let now = 1_000_000i64;
        assert!(is_expired(now - 1, 0, now));
        assert!(!is_expired(now, 0, now));
    }

    #[test]
    fn cache_key_video_search_matches_legacy_format() {
        for (q, n) in [("lofi", 15usize), ("café", 1usize), ("", 0usize)] {
            assert_eq!(
                cache_key_video_search(q, n),
                get_cache_key(&format!("video:{}:{}", q, n))
            );
        }
        let long = "x".repeat(200);
        assert_eq!(
            cache_key_video_search(&long, 99),
            get_cache_key(&format!("video:{}:{}", long, 99))
        );
    }

    #[test]
    fn cache_key_channel_videos_matches_legacy_format() {
        for (h, n) in [
            ("@Handle", 5usize),
            ("channel/UCabc", 10usize),
            ("", 1usize),
        ] {
            assert_eq!(
                cache_key_channel_videos(h, n),
                get_cache_key(&format!("channel:{}:{}", h, n))
            );
        }
        let long = "y".repeat(100);
        assert_eq!(
            cache_key_channel_videos(&long, 20),
            get_cache_key(&format!("channel:{}:{}", long, 20))
        );
    }
}
